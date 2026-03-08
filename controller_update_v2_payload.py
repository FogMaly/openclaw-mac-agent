import os

CONTROL_TOKEN = "control-secret-token"

# This script is meant to be executed in the controller service project directory.
# It will:
#   1) Create/overwrite a `controller_exec_v2_example.py` file that implements /api/exec_v2.
#   2) Find a main application file (controller_main.py, main.py, or app.py).
#   3) Append router mounting code so that /api/exec_v2 is registered on the FastAPI app.

handler_code = """\
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel
from typing import List, Optional
import subprocess

CONTROL_TOKEN = "{token}"

router = APIRouter()


class ExecRequest(BaseModel):
    agent_id: str
    kind: str
    command: str
    args: List[str] = []
    cwd: Optional[str] = None
    timeout: int = 30000
    control_token: str


class ExecResponse(BaseModel):
    stdout: Optional[str] = None
    stderr: Optional[str] = None
    exit_code: Optional[int] = None
    error: Optional[str] = None


@router.post("/api/exec_v2", response_model=ExecResponse)
async def exec_v2(req: ExecRequest) -> ExecResponse:
    if req.control_token != CONTROL_TOKEN:
        raise HTTPException(status_code=403, detail="Invalid control_token")

    try:
        proc = subprocess.run(
            [req.command, *req.args],
            cwd=req.cwd or None,
            capture_output=True,
            text=True,
            timeout=req.timeout / 1000,
        )
        return ExecResponse(
            stdout=proc.stdout,
            stderr=proc.stderr,
            exit_code=proc.returncode,
            error=None,
        )
    except subprocess.TimeoutExpired as e:
        return ExecResponse(
            stdout=e.stdout,
            stderr=e.stderr,
            exit_code=None,
            error="Timeout",
        )
    except Exception as e:  # noqa
        return ExecResponse(
            stdout=None,
            stderr=str(e),
            exit_code=None,
            error="Exception",
        )
""".format(token=CONTROL_TOKEN)

# 1) Write handler file
handler_path = "controller_exec_v2_example.py"
with open(handler_path, "w", encoding="utf-8") as f:
    f.write(handler_code)
print(f"[controller-update-v2] wrote {handler_path}")

# 2) Locate main application file
candidates = ["controller_main.py", "main.py", "app.py"]
main_file = None
for name in candidates:
    if os.path.exists(name):
        main_file = name
        break

if not main_file:
    print("[controller-update-v2] ERROR: could not find main app file (controller_main.py / main.py / app.py). Please wire the router manually.")
    raise SystemExit(1)

print(f"[controller-update-v2] found main app file: {main_file}")

with open(main_file, "r", encoding="utf-8") as f:
    content = f.read()

snippet = (
    "from controller_exec_v2_example import router as exec_v2_router\n"
    "app.include_router(exec_v2_router)\n"
)

if "controller_exec_v2_example" in content or "exec_v2_router" in content:
    print("[controller-update-v2] router already appears to be wired; skipping append.")
else:
    with open(main_file, "a", encoding="utf-8") as f:
        f.write("\n\n# ----- exec_v2 router wiring -----\n")
        f.write(snippet)
    print(f"[controller-update-v2] appended exec_v2 router wiring to {main_file}.")

print("[controller-update-v2] done. Please restart the controller service so /api/exec_v2 becomes active.")
