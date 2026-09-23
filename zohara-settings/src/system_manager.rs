use sysinfo::System;

pub struct SystemManager {
    sys: System,
}

impl SystemManager {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys }
    }

    pub fn get_os_version(&self) -> String {
        System::os_version().unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn get_kernel_version(&self) -> String {
        System::kernel_version().unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn get_cpu_model(&self) -> String {
        if let Some(cpu) = self.sys.cpus().first() {
            cpu.brand().to_string()
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_memory_total(&self) -> String {
        let total_kb = self.sys.total_memory() / 1024;
        format!("{:.1} GB", total_kb as f64 / 1024.0 / 1024.0)
    }
}
