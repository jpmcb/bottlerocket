%global goproject github.com/kubernetes
%global gorepo cloud-provider-aws
%global goimport %{goproject}/%{gorepo}

%global gover 1.22.3
%global rpmver %{gover}

Name: %{_cross_os}aws-ecr-credential-provider
Version: %{rpmver}
Release: 1%{?dist}
Summary: Container image registry credential provider for AWS ECR
License: Apache-2.0
URL: https://github.com/kubernetes/cloud-provider-aws
Source0: https://%{goimport}/archive/v%{gover}/v%{gover}.tar.gz

#BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}
export CGO_LDFLAGS="-Wl,-z,now"
make ecr-credential-provider
# go build -ldflags="${GOLDFLAGS}" -o nvidia-device-plugin ./cmd/nvidia-device-plugin/

%install
install -d %{buildroot}%{_cross_bindir}
install -d %{buildroot}%{_cross_unitdir}
install -p -m 0755 ecr-credential-provider %{buildroot}%{_cross_bindir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}

%files
%license LICENSE
%{_cross_attribution_file}
