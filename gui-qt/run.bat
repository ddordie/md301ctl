@echo off
rem 首次运行会建虚拟环境并装 PySide6，之后直接起界面
setlocal
cd /d %~dp0
if not exist .venv\Scripts\python.exe (
  echo 正在创建虚拟环境...
  uv venv --python 3.11 .venv || python -m venv .venv
  .venv\Scripts\python.exe -m pip install -r requirements.txt
)
.venv\Scripts\pythonw.exe app.py
