# ai4ose-tg-rcore-tutorial-syscall-doom

This crate is the Doom-oriented syscall layer used by the publishable `ai4ose-tg-rcore-tutorial-ch8-doom` experiment crate.

It extends the original tutorial syscall surface with the interfaces needed by the Doom port:

- framebuffer metadata export
- framebuffer present
- non-blocking input polling
- `lseek`
- `sbrk`

It is published separately so the main ch8 Doom crate can depend on a public registry package instead of unpublished local workspace edits.
