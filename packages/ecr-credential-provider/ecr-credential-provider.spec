%global goproject github.com/kubernetes
%global gorepo cloud-provider-aws
%global goimport %{goproject}/%{gorepo}

%global gover 1.22.3
%global rpmver %{gover}

%global gitrev 45289e33c0fde6b6779cb80b60f6896354c9a860
%global shortrev %(c=%{gitrev}; echo ${c:0:7})

Name: %{_cross_os}aws-ecr-credential-provider
Version: %{rpmver}
Release: 1%{?dist}
Summary: Container image registry credential provider for AWS ECR
License: Apache-2.0
URL: https://github.com/kubernetes/cloud-provider-aws
Source0: %{gorepo}-%{shortrev}-bundled.tar.gz

#BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%setup -n %{gorepo}-%{gitrev} -q

%build
%set_cross_go_flags

#export CGO_LDFLAGS="-Wl,-z,now"
#export GO_LDFLAGS="-w -s -X k8s.io/component-base/version.gitVersion='v1.23.1'"
make ecr-credential-provider
#go build -trimpath -ldflags="${GO_LDFLAGS}" -o ecr-credential-provider ./cmd/ecr-credential-provider/*.go

%install
install -d %{buildroot}%{_cross_bindir}
install -d %{buildroot}%{_cross_unitdir}
install -p -m 0755 ecr-credential-provider %{buildroot}%{_cross_bindir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}

%files
%license LICENSE
%{_cross_attribution_file}
