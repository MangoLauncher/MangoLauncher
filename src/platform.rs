use std::path::PathBuf;

pub fn get_default_java_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        paths.push(PathBuf::from("C:\\Program Files\\Java"));
        paths.push(PathBuf::from("C:\\Program Files (x86)\\Java"));
    }
    
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        paths.push(PathBuf::from("/System/Library/Java/JavaVirtualMachines"));
    }
    
    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/lib/jvm"));
        paths.push(PathBuf::from("/usr/java"));
        paths.push(PathBuf::from("/opt/java"));
    }
    
    paths
}

pub fn get_classpath_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
} 