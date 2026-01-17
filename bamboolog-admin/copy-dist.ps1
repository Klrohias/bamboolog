Remove-Item -Path "../bamboolog/admin-dist/*" -Recurse -Force
Copy-Item -Path "dist/*" -Destination "../bamboolog/admin-dist/" -Recurse -Force

New-Item -Path "../bamboolog/admin-dist/.gitkeep" -ItemType File -Force
Set-Content -Path "../bamboolog/admin-dist/.gitignore" -Value "*`n!.gitkeep"
