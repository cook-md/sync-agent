#!/bin/bash
# Post-installation tasks for Cook Sync

# Update icon cache so system tray and desktop environments can find icons
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor/ 2>/dev/null || true
fi

# Update desktop database so application menus pick up changes
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications/ 2>/dev/null || true
fi

exit 0
