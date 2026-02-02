# UniEdit Todo

## General

- [x] Reusable Design
- [x] "System" option to decide light/dark mode
- [ ] Create Main Menu
  - [ ] Page you go to changes depending on what file you put in on main menu
- [x] Sidebar
  - [x] Recent Activity
  - [x] Converter/Screen List
  - [ ] Change checkmark close/open sidebar to a small arrow.

## Screens

- [x] Txt, Md
- [ ] Word Docs, ect.
- [x] Img Processing
- [ ] Video Processing
- [ ] JSON Processing
- [ ] CSV, Excel, ect.
- [ ] PDF
- [ ] Zip/Unzip
- [ ] Code Editor?

## Converters

- [ ] Video to Gif, Gif to Video
- [ ] Video Compressor
- [x] Img to Img of diff type
- [ ] Video to Audio File

## Debugging

#### Text Editor

- [ ] Cannot highlight and scroll at the same time? - Hard
- [ ] Can't Combine Formatting - Medium
- [ ] At 20 Font size with Sans font, code block doesn't format correctly.
- [ ] Save warning dialogue does not show up on application close
- [x] Code block cursor desync
- [x] Code block desync with words right after/before code block
- [x] Can click to non-existant line in markdown mode, desyncing cursor.
- [x] Cannot use strikethrough keyboard shortcut.
- [x] Punctuation after Underlined/Bolding/Italics doesn't render properly
- [x] If there is a asterisk MUCH after the last italicized word, It will think it connected
  - Basically, make sure to not check asterisks that are actively being used in words.
- [x] Bolding is barely noticable
  - Seems like just turning the markdown text into same color as the plaintext will help a lot.
- [x] Can't put punctuation after subscript or superscript.
- [x] Features in top tool bar are not aligned.
- [x] When using headers, cursor gets disconnected from the actual position
- [x] Change in font size (E.g Bold & Headers) make code block background out of position

#### Other
- [x] Sidebar down arrow shows as empty square
- [x] Top bar "View" should not show text editor options when not on text editor

## Features

#### Text Editor

- [x] Keyboard Shortcuts
- [x] Markdown Loader
- [x] Download/Save Files
- [x] Bold, Italics, Underline, Strikethrough
  - [x] Make Bold stand out more
- [x] Font, Font Size
- [x] Headers
  - [x] Button to add headers
- [x] Superscript, Subscript
- [x] Clickable Links
- [x] Hover features in toolbar to know what they are (E.g hover B to show "bold")
- [x] Show what file is being edited
- [x] Turn off Toolbar
- [x] Make Code blocks more readable.
  - [x] Add titles to code blocks (E.g "bash" or "rust")
  - [x] Add newlines after and before code in code block.
- [x] Showing saved/unsaved
- [ ] Latex?

#### Image Editor

- [ ] Layer system 
  - [ ] Add
  - [ ] Reorder
  - [ ] Merge
  - [ ] Opacity
- [ ] Selection tools
  - [ ] Ellipse
  - [ ] Free-hand lasso
  - [ ] Magic wand
- [ ] Gradient tool
  - [ ] Linear
  - [ ] Radial
- [ ] Performance
  - [ ] Brush is slightly laggy, make more smooth
  - [ ] Increase performance of filters on bigger images (E.g Blur)
  - [ ] General performance and usability improvements
- [ ] Color picker
  - [ ] Recent colors option in color picker
  - [ ] Bigger/Clearer Color picker
  - [ ] Color code to color option
- [ ] More brush presets or options
  - [ ] Custom brush shapes?
- [ ] Copy/paste/cut within selection
- [ ] Pattern/texture fill
- [ ] Perspective/affine warp
- [ ] Color code to color option
- [ ] Edge detection filter
- [ ] Canvas size (extend without scaling)
- [ ] Export with metadata
- [ ] Export to other image types
- [ ] Snap to grid for text/crop
- [ ] Filter preview before apply
- [ ] Better toolbar organization
- [ ] Clearer option selection (instead of constant dropdowns, e.g Filter options)
- [ ] Drag-and-drop image open
- [ ] Improve button look when the screen/resolution is smaller
- [ ] Images show up in recent files

#### CONVERTER: Img 2 Img
- [ ] Keyboard Shortcuts
- [x] Display Error (For example, width > 256 for ico conversion)

## Other

- [x] Create Todo List
- [ ] Clear/Remove recent files

## Testing

- [ ] Create Testing Framework
