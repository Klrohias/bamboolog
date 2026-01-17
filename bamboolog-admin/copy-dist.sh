#!/bin/bash

rm -rf ../bamboolog/admin-dist/*
cp -r dist/* ../bamboolog/admin-dist/

touch ../bamboolog/admin-dist/.gitkeep
echo "*" > ../bamboolog/admin-dist/.gitignore
echo "!.gitkeep" >> ../bamboolog/admin-dist/.gitignore
