import subprocess
import os
from PySide6.QtCore import QObject, Slot, Signal, Property

class SystemManager(QObject):
    specsChanged = Signal()

    def __init__(self):
        super().__init__()
        # Cache specs at startup (they don't change at runtime)
        self._kernel = self._read_kernel()
        self._os_name = self._read_os_field("PRETTY_NAME")
        self._codename = self._read_os_field("ZOHARA_CODENAME")
        self._version = self._read_os_field("VERSION")
        self._cpu = self._read_cpu()
        self._ram = self._read_ram()
        self._gpus = self._read_gpus()

    def _read_os_field(self, field):
        try:
            with open("/etc/os-release") as f:
                for line in f:
                    if line.startswith(field + "="):
                        return line.split("=", 1)[1].strip().strip('"')
        except:
            pass
        return ""

    def _read_kernel(self):
        try:
            return subprocess.check_output(["uname", "-r"], text=True).strip()
        except:
            return "Unknown"

    def _read_cpu(self):
        try:
            model = ""
            cores = 0
            threads = 0
            with open("/proc/cpuinfo") as f:
                for line in f:
                    if "model name" in line and not model:
                        model = line.split(":", 1)[1].strip()
                    if "processor" in line:
                        threads += 1
            # Physical core count
            try:
                out = subprocess.check_output(
                    ["lscpu", "--parse=CORE"], text=True, stderr=subprocess.DEVNULL
                )
                cores = len(set(
                    l.strip() for l in out.splitlines()
                    if l.strip() and not l.startswith("#")
                ))
            except:
                cores = threads
            return {"model": model, "cores": cores, "threads": threads}
        except:
            return {"model": "Unknown", "cores": 0, "threads": 0}

    def _read_ram(self):
        try:
            with open("/proc/meminfo") as f:
                for line in f:
                    if "MemTotal" in line:
                        kb = int(line.split()[1])
                        return f"{kb / 1024 / 1024:.1f} GB"
        except:
            pass
        return "Unknown"

    def _read_gpus(self):
        """Returns list of GPUs, distinguishing discrete vs integrated."""
        try:
            out = subprocess.check_output(
                ["lspci", "-k"], text=True, stderr=subprocess.DEVNULL
            )
            gpus = []
            lines = out.splitlines()
            for i, line in enumerate(lines):
                if "VGA compatible" in line or "3D controller" in line or "Display controller" in line:
                    name_part = line.split(":", 2)[-1].strip()
                    name_lower = name_part.lower()
                    # Heuristic: Intel HD/UHD/Iris = iGPU, everything else = dGPU
                    is_igpu = (
                        "intel" in name_lower and
                        any(x in name_lower for x in ["hd graphics", "uhd graphics", "iris", "xe graphics"])
                    )
                    gpus.append({
                        "name": name_part,
                        "type": "Integrated" if is_igpu else "Discrete"
                    })
            return gpus
        except:
            return []

    @Slot(result=str)
    def getKernelVersion(self):
        return self._kernel

    @Slot(result=str)
    def getOsVersion(self):
        return self._os_name

    @Slot(result=str)
    def getCodename(self):
        return self._codename

    @Slot(result=str)
    def getOsVersionId(self):
        return self._version

    @Slot(result=str)
    def getCpuModel(self):
        return self._cpu["model"]

    @Slot(result=int)
    def getCpuCores(self):
        return self._cpu["cores"]

    @Slot(result=int)
    def getCpuThreads(self):
        return self._cpu["threads"]

    @Slot(result=str)
    def getMemoryTotal(self):
        return self._ram

    @Slot(result=list)
    def getGpus(self):
        return self._gpus

    @Slot(result=str)
    def copySpecsToClipboard(self):
        gpus_str = "\n".join([f"  {g['type']} GPU: {g['name']}" for g in self._gpus])
        text = (
            f"Zohara OS {self._version} ({self._codename})\n"
            f"Kernel: {self._kernel}\n"
            f"CPU: {self._cpu['model']} ({self._cpu['cores']}C / {self._cpu['threads']}T)\n"
            f"RAM: {self._ram}\n"
            f"GPU:\n{gpus_str}"
        )
        return text
