#!/bin/bash
# Mimic a theme that left a second panel behind: a floating dock with the app icons, and none on the main bar.
mkdir -p ~/.local/share/icons/FakeTheme/16 ~/.local/share/color-schemes
printf '[Icon Theme]\nName=Fake Icons\nDirectories=16\n' > ~/.local/share/icons/FakeTheme/index.theme
printf '[General]\nName=Fake Night\n' > ~/.local/share/color-schemes/FakeNight.colors
JS='var m=null; panels().forEach(function(p){ p.widgets().forEach(function(w){ if(w.type=="org.kde.plasma.icontasks"||w.type=="org.kde.plasma.taskmanager") w.remove(); }); });
var d=new Panel(); d.location="bottom"; d.height=48; d.alignment="center"; d.addWidget("org.kde.plasma.icontasks");'
dbus-send --session --type=method_call --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript "string:$JS"
echo messed
