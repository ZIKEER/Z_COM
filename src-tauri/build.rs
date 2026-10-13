fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    let build_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    println!("cargo:rustc-env=Z_COM_BUILD_TIMESTAMP={build_timestamp}");
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("icons/icon.ico")
        // 与原 tauri-build 嵌入的清单保持一致，供 J-Link DLL 等第三方对话框使用 Common-Controls v6
        .set_manifest(
            r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
</assembly>"#,
        )
        .compile()
        .expect("无法编译 Windows 图标资源");
}
