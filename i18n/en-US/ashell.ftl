## Seed translation catalog. New keys land here first; other locales inherit
## via Fluent's negotiation + fallback.

app-name = ashell

## Updates module
updates-up-to-date = Up to date ;)
updates-available =
    { $count ->
        [one] { $count } Update available
       *[other] { $count } Updates available
    }
updates-button-update = Update
updates-button-check-now = Check now

## Media player module
media-player-not-connected = Not connected to MPRIS service
media-player-heading = Players
media-player-loading-cover = Loading cover...
media-player-no-title = No Title
media-player-unknown-artist = Unknown Artist
media-player-unknown-album = Unknown Album

## Password / network connection dialog
password-dialog-open-network-title = Open network
password-dialog-authentication-required-title = Authentication required
password-dialog-open-network-warning =
    "{ $ssid }" is an open network. Data sent over this connection may be visible to others.
    Do you want to connect anyway?
password-dialog-insert-password = Insert password to connect to: { $ssid }
password-dialog-cancel = Cancel
password-dialog-confirm = Confirm

## OSD
osd-airplane-toggle =
    { $state ->
        [on] Airplane mode turned on
       *[off] Airplane mode turned off
    }
osd-idle-inhibitor-toggle =
    { $state ->
        [on] Idle inhibitor turned on
       *[off] Idle inhibitor turned off
    }
