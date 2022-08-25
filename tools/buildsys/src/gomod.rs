/*!
Packages using the Go programming language may have upstream tar archives that
include only the source code of the project, but not the source code of any
dependencies. The Go programming language promotes the use of "modules" for
dependencies and projects adopting modules will provide go.mod and go.sum
files.

This module extends the functionality of `packages.metadata.build-package.external-files`
and provides the ability to retrieve and validate dependencies
declared using Go modules given a tar archive containing a go.mod and go.sum.

The location where dependencies are retrieved from are controlled by the
standard environment variables employed by the Go tool: GOPROXY, GOSUMDB, and
GOPRIVATE.

 */

pub(crate) mod error;
use error::Result;

use super::manifest;
use duct::cmd;
use snafu::{ensure, OptionExt, ResultExt};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Output;

pub(crate) struct GoMod;

impl GoMod {
    pub(crate) fn vendor(
        root_dir: &Path,
        package_dir: &Path,
        gomod: &manifest::ExternalFile,
    ) -> Result<()> {
        let url_file_name = extract_file_name(&gomod.url)?;
        let path_arg = &gomod.path.as_ref().unwrap_or(&url_file_name);
        ensure!(path_arg.components().count() == 1, error::InputFileSnafu);

        let path = package_dir.join(path_arg);
        ensure!(path.is_file(), error::InputFileBadSnafu { path });
        let mod_dir = gomod.bundle_path.as_ref().context(error::ModDirSnafu)?;
        let output_path_arg = gomod
            .bundle_output_path
            .as_ref()
            .context(error::OutputDirSnafu)?;
        // let output_path = package_dir.join(output_path_arg);

        // Return early if the output path exists and is a file, assuming it's already been built
        // if output_path.exists() && output_path.is_file() {
        //    return Ok(());
        // }

        // Our SDK and toolchain are picked by the external `cargo make` invocation.
        let sdk = getenv("BUILDSYS_SDK_IMAGE")?;

        // Several Go variables control proxying
        let mut goproxy = go_env("GOPROXY").unwrap_or_else(|| "".to_string());
        if goproxy.ends_with('\n') {
            goproxy.pop();
        }
        let mut gosumdb = go_env("GOSUMDB").unwrap_or_else(|| "".to_string());
        if gosumdb.ends_with('\n') {
            gosumdb.pop();
        }
        let mut goprivate = go_env("GOPRIVATE").unwrap_or_else(|| "".to_string());
        if goprivate.ends_with('\n') {
            goprivate.pop();
        }

        let args = DockerGoArgs {
            module_path: package_dir,
            sdk_image: sdk,
            go_mod_cache: &root_dir.join(".gomodcache"),
            command: format!(
                "tar xf {input} &&
                pushd {moddir} &&
                export GOPROXY={goproxy} &&
                export GOSUMDB={gosumdb} &&
                export GOPRIVATE={goprivate} &&
                go list -mod=readonly ./... >/dev/null && go mod vendor &&
                popd &&
                tar czf {output} {moddir} && 
                rm -rf {moddir}",
                input = path_arg.to_string_lossy(),
                moddir = mod_dir.to_string_lossy(),
                goproxy = goproxy,
                gosumdb = gosumdb,
                goprivate = goprivate,
                output = output_path_arg.to_string_lossy(),
            ),
        };
        docker_go(root_dir, &args)?;

        Ok(())
    }
}

fn extract_file_name(url: &str) -> Result<PathBuf> {
    let parsed = reqwest::Url::parse(url).context(error::InputUrlSnafu { url })?;
    let name = parsed
        .path_segments()
        .context(error::InputFileBadSnafu { path: url })?
        .last()
        .context(error::InputFileBadSnafu { path: url })?;
    Ok(name.into())
}

struct DockerGoArgs<'a> {
    module_path: &'a Path,
    sdk_image: String,
    go_mod_cache: &'a Path,
    command: String,
}

/// Run `docker-go` with the specified arguments.
fn docker_go(root_dir: &Path, dg_args: &DockerGoArgs) -> Result<Output> {
    let args = vec![
        "--module-path",
        dg_args
            .module_path
            .to_str()
            .context(error::InputFileSnafu)?,
        "--sdk-image",
        &dg_args.sdk_image,
        "--go-mod-cache",
        dg_args
            .go_mod_cache
            .to_str()
            .context(error::InputFileSnafu)?,
        "--command",
        &dg_args.command,
    ];
    let arg_string = args.join(" ");
    let program = root_dir.join("tools/docker-go");
    println!("program: {}", program.to_string_lossy());
    let output = cmd(program, args)
        .stderr_to_stdout()
        .stdout_capture()
        .unchecked()
        .run()
        .context(error::CommandStartSnafu)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", &stdout);
    ensure!(
        output.status.success(),
        error::DockerExecutionSnafu { args: arg_string }
    );
    Ok(output)
}

/// Run `go env` with the specified argument.
fn go_env(var: &str) -> Option<String> {
    let args = vec!["env", var];
    let output = match cmd("go", args)
        .stderr_to_stdout()
        .stdout_capture()
        .unchecked()
        .run()
    {
        Ok(v) => v,
        Err(_) => return None,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", &stdout);
    output.status.success().then(|| stdout.to_string())
}

/// Retrieve a BUILDSYS_* variable that we expect to be set in the environment,
/// and ensure that we track it for changes, since it will directly affect the
/// output.
fn getenv(var: &str) -> Result<String> {
    println!("cargo:rerun-if-env-changed={}", var);
    env::var(var).context(error::EnvironmentSnafu { var })
}
