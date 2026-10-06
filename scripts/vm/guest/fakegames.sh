#!/bin/bash
# Test fixture: a fake Steam library (plus a second library path), a fake Lutris game, a fake Heroic Epic game.
rm -rf /tmp/lib2 ~/.local/share/Steam/steamapps ~/.config/lutris/games
mkdir -p /tmp/lib2/steamapps ~/.local/share/Steam/steamapps ~/.config/lutris/games
printf '"libraryfolders"\n{\n"1"\n{\n"path"\t"/tmp/lib2"\n}\n}\n' > ~/.local/share/Steam/steamapps/libraryfolders.vdf
acf(){ printf '"AppState"\n{\n"appid"\t"%s"\n"name"\t"%s"\n"StateFlags"\t"%s"\n}\n' "$2" "$3" "$4" > "$1/steamapps/appmanifest_$2.acf"; }
acf ~/.local/share/Steam 440 "Team Fortress 2" 4
acf /tmp/lib2 400 "Portal" 4
acf /tmp/lib2 9 "Proton 9.0" 4
acf /tmp/lib2 77 "Half-Finished Download" 1026
touch ~/.config/lutris/games/hades-1650000000.yml ~/.config/lutris/games/the-witcher-3-1650000001.yml
H=~/.config/heroic/legendaryConfig/legendary
mkdir -p "$H"
echo '{"abc":{"title":"Rocket League"}}' > "$H/installed.json"
echo fixtures ready
