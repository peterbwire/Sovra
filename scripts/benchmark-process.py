"""One isolated, timed child. POSIX wait4 peak RSS belongs to this child only."""
import json
import subprocess
import sys
import time

try:
    import resource
except ImportError:
    resource = None

start = time.perf_counter()
child = subprocess.run(sys.argv[1:], capture_output=True, timeout=30, check=False)
elapsed = (time.perf_counter() - start) * 1000
rss = None
if resource is not None and sys.platform in ("linux", "darwin"):
    raw = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    rss = int(raw if sys.platform == "darwin" else raw * 1024)
print(json.dumps({
    "wall_ms": elapsed,
    "peak_rss_bytes": rss,
    "status": child.returncode,
    "stdout": child.stdout.decode("utf-8", errors="strict"),
    "stderr": child.stderr.decode("utf-8", errors="strict"),
}))
