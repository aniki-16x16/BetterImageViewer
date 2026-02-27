fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // 只有当 icon.ico 存在时才设置图标，避免编译报错
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
            res.compile().unwrap();
        }
    }
}
