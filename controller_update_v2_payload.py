import os, textwrap

CONTROL_TOKEN = "control-secret-token"

handler_code = textwrap.dedent(f"""
    """Exec v2 handler 示例

    将此文件集成到你的 FastAPI/Starlette 应用中：

      from controller_exec_v2_example import router as exec_v2_router
      app.include_router(exec_v2_router)

    或者手动复制 exec_v2 函数逻辑到现有 router 中。
    """

    from fastapi import APIRouter, HTTPException
    from pydantic import BaseModel
    from typing import List, Optional
    import subprocess

    CONTROL_TOKEN = "{CONTROL_TOKEN}"

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

        # TODO: 根据 req.kind/command/cwd 做更多安全控制

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
""")

# 1) 写入 handler 文件
handler_path = "controller_exec_v2_example.py"
with open(handler_path, "w", encoding="utf-8") as f:
    f.write(handler_code)
print(f"[controller-update-v2] 写入 {handler_path}")

# 2) 找主程序文件：常见三个名字里挑一个存在的
candidates = ["controller_main.py", "main.py", "app.py"]
main_file = None
for name in candidates:
    if os.path.exists(name):
        main_file = name
        break

if not main_file:
    print("[controller-update-v2] ❌ 没找到主程序文件（controller_main.py / main.py / app.py 都不存在），需要你手动接入 router。")
    raise SystemExit(1)

print(f"[controller-update-v2] 发现主程序文件: {main_file}")

with open(main_file, "r", encoding="utf-8") as f:
    content = f.read()

snippet = (
    "from controller_exec_v2_example import router as exec_v2_router\n"
    "app.include_router(exec_v2_router)\n"
)

if "controller_exec_v2_example" in content or "exec_v2_router" in content:
    print("[controller-update-v2] 已检测到相关代码，跳过追加。")
else:
    # 简单做法：直接在文件末尾追加
    with open(main_file, "a", encoding="utf-8") as f:
        f.write("\n\n# ----- exec_v2 路由挂载 -----\n")
        f.write(snippet)
    print(f"[controller-update-v2] 已在 {main_file} 末尾追加 exec_v2 路由挂载代码。")

print("[controller-update-v2] 完成。请按你原来的方式重启控制端服务，使 /api/exec_v2 生效。")
