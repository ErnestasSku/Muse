# Muse

Image reference app for desktop and mobile.

## Current state

It can load images by dragging them. (Images must be files on the system).
It is possible to manually sync state between different applications.

## MVP

Working desktop and android application.
Can drag images from desktop to application.
Can sync state between different clients. (Done manually)

### Nice to have/Blocked:

- [ ] Draging images/data from browser into the app.
- [ ] Pasting images from clipboard.

This is currently not doable due to winit's limitation.

### Future work:

- [ ] Custom GIF widget
  - [ ] Pausing/Resuming functionality
  - [ ] Next/Previous frame functionality
  - [ ] Frame preview
- [ ] Improve UI
- [ ] Allow to enable/disable syncing from the UI
- [ ] Implement automatic image sharing between clients
- [ ] Remove unsafety:
  - [ ] Implement a Toast/Notification widget for error reporting
  - [ ] Instead of unwraping results, use the widget to report errors
- [ ] Improved image widget
  - [ ] Selected outline (with edge bubbles)
  - [ ] Resizing/Scaling
  - [ ] Rotation
