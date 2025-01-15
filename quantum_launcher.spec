Name:           quantum-launcher
Version:        0.3.1
Release:        %autorelease
Summary:        Simple Minecraft Launcher written in Rust

License:        GPLv3
URL:            https://mrmayman.github.io/quantumlauncher
Source:        {{{ git_dir_pack }}}

BuildRequires:  rust cargo openssl-devel perl

%global _description %{expand:
A simple Minecraft Launcher written in Rust.}

%description %{_description}

%prep
{{{ git_dir_setup_macro }}}
cargo fetch

%build
cargo build --profile release

%install
install -Dm755 target/release/quantum_launcher %{buildroot}%{_bindir}/quantum-launcher

%files
%license LICENSE*
%doc README.md
%{_bindir}/quantum-launcher

%changelog
%autochangelog