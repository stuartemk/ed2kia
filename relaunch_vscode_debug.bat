@echo off
REM Script para relanzar VS Code con depuración habilitada en el puerto 9222
REM Este script permite conectar las DevTools de Chrome a VS Code

echo ========================================
echo Relanzando VS Code con depuración MCP
echo ========================================
echo.

REM Cierra todas las instancias de VS Code para asegurar un reinicio limpio
echo Cerrando instancias existentes de VS Code...
taskkill /F /IM code.exe 2>nul

REM Espera un momento para asegurar que VS Code se cierre completamente
timeout /t 2 /nobreak >nul

REM Inicia VS Code con el puerto de depuración habilitado
echo Iniciando VS Code con depuración en puerto 9222...
start "" "C:\Users\cualo\AppData\Local\Programs\Microsoft VS Code\Code.exe" --remote-debugging-port=9222 "%~dp0"

echo.
echo ========================================
echo VS Code se ha iniciado correctamente
echo Puerto de depuración: 9222
echo ========================================
echo.
echo Ahora puedes:
echo   1. Abrir Chrome con: start chrome "chrome://inspect/#devices" --remote-debugging-port=9222
echo   2. Usar las DevTools MCP para depurar extensiones
echo   3. Presionar F5 en VS Code para ejecutar la extensión en modo depuración
echo.
