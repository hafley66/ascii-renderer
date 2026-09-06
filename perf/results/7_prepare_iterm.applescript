-- Creates a dedicated raw session. Font changes stay local to that session.
tell application id "com.googlecode.iterm2"
 set w to create window with default profile command "/bin/bash --noprofile --norc"
 tell current session of w to set name to "ascii-renderer raw iTerm2 repro"
 activate
 select w
end tell
tell application "System Events" to tell process "iTerm2"
 set marker to value of attribute "AXMenuItemMarkChar" of menu item "Size Changes Update Profile" of menu "View" of menu bar item "View" of menu bar 1
 if marker is not missing value and marker is not "" then error "Disable Size Changes Update Profile for session-local zoom before running the repro"
 repeat 17 times
  click menu item "Make Text Smaller" of menu "View" of menu bar item "View" of menu bar 1
 end repeat
end tell
tell application id "com.googlecode.iterm2"
 tell current session of w
  set columns to 1718
  set rows to 358
 end tell
 return id of w
end tell
