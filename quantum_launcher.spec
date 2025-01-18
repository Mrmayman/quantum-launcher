Name:           quantum-launcher
Version:        0.3.1
Release:        1%{?dist}
Summary:        Simple Minecraft Launcher written in Rust

License:        GPLv3
URL:            https://mrmayman.github.io/quantumlauncher
Source:        {{{ git_dir_pack }}}

BuildRequires:  rust cargo perl

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
install -Dm644 assets/freedesktop/quantum-launcher.desktop %{buildroot}/usr/share/applications/quantum-launcher.desktop
install -Dm644 assets/icon/ql_logo.png %{buildroot}/usr/share/pixmaps/ql_logo.png
install -Dm644 assets/freedesktop/quantum-launcher.metainfo.xml %{buildroot}/usr/share/metainfo/quantum-launcher.metainfo.xml

%files
%license LICENSE
%doc README.md
%{_bindir}/quantum-launcher
/usr/share/applications/quantum-launcher.desktop
/usr/share/pixmaps/ql_logo.png
/usr/share/metainfo/quantum-launcher.metainfo.xml

%changelog
%autochangelog
