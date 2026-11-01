@echo off
setlocal
for /f "usebackq tokens=*" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "PRISME_VS=%%i"
if not defined PRISME_VS exit /b 1
call "%PRISME_VS%\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1
if not exist .build-plugins mkdir .build-plugins
cl /nologo /std:c11 /LD /MT /O2 /utf-8 /Fo.build-plugins\invert.obj plugin-sdk\native\invert.c /link /OUT:plugin-sdk\native\invert.dll /IMPLIB:.build-plugins\invert.lib
if errorlevel 1 exit /b 1
cl /nologo /std:c11 /LD /MT /O2 /utf-8 /Fo.build-plugins\negative.obj plugin-sdk\pfl\examples\negative.c /link /OUT:plugin-sdk\pfl\examples\negative.pfl /IMPLIB:.build-plugins\negative.lib
exit /b %errorlevel%
