# alpha

<img width="1987" height="1629" alt="Screenshot 2026-10-27 at 00-08-53" src="https://github.com/user-attachments/assets/637b312e-c563-484b-be2b-9eea6443571f" />

a markdown editor for mac. it edits markdown, that's the whole pitch.
written in rust. we're not sorry.

## get it

grab the dmg from [releases](https://github.com/butter-computer/alpha/releases/latest). apple silicon only.

not notarized yet, so the first open needs system settings > privacy & security > open anyway, or:

`xattr -dr com.apple.quarantine /Applications/Alpha.app`

## what's in it

- live markdown that hides the syntax while you write
- lists, checkboxes and code blocks
- cmd+k for commands, `/` for blocks
- search, and export to md or html
- follows light/dark

## build it yourself

needs rust and xcode.

```sh
cargo run --release
scripts/bundle-mac.sh   # Alpha.app
scripts/make-dmg.sh     # the dmg
```

## shortcuts

cmd+k commands, cmd+f search, cmd+, settings, cmd+= / cmd+- text size
