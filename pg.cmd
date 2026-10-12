@echo off
rem The cmd.exe entry point for pg, so `pg start` works without a .\ prefix here.
rem
rem It hands over to pg.ps1 with the execution policy bypassed for this one run, which is what makes
rem the same file work from a locked-down PowerShell too.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pg.ps1" %*
