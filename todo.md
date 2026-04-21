# UniEdit Todo

## General

- [ ] Editable keyboard shortcuts per page

## Screens

- [ ] Code Editor (.rs, .py, .js, .ts, .c, .cpp, .go, .java, .sh, .css, .sql)
- [ ] Spreadsheet Editor (.csv, .tsv, .xlsx, .xls, .ods)
- [ ] PDF Editor (.pdf)
- [ ] HTML Editor (.html, .htm)
- [ ] Archive Manager (.zip, .tar, .gz, .tar.gz, .7z, .bz2)
- [ ] Audio Editor (.mp3, .wav, .flac, .ogg, .aac, .m4a)
- [ ] Video Editor (.mp4, .mkv, .avi, .mov, .webm)
- [ ] Hex Editor (any binary file)
- [ ] Font Viewer (.ttf, .otf, .woff, .woff2)

## Converters

- [ ] Video to Video (.mp4, .mkv, .avi, .mov, .webm, .flv, .wmv)
- [ ] Video Compressor (.mp4, .mkv, .mov, .webm)
- [ ] Video to GIF / GIF to Video (.mp4/.webm -> .gif, .gif -> .mp4/.webm)
- [ ] Video to Audio (.mp4, .mkv, .avi, .mov -> .mp3, .wav, .flac, .ogg, .aac)
- [ ] Subtitle Converter (.srt, .vtt, .ass, .ssa)
- [ ] Audio Converter (.mp3, .wav, .flac, .ogg, .aac, .m4a)
- [ ] Audio Compressor (.wav, .flac, .aac, .mp3)
- [ ] Document Converter (.docx, .odt -> .pdf, .txt, .md, .html, .rtf)
- [ ] PDF Converter (.pdf -> .txt, .docx, .html, per-page image exports)
- [ ] Spreadsheet Converter (.csv, .tsv, .xlsx, .ods -> any of those, includes .json export)
- [ ] Markdown Converter (.md -> .html, .pdf, .docx, .txt)
- [ ] SVG Rasterizer (.svg -> .png, .jpg, .webp)
- [ ] Image to PDF / PDF to Images (.jpg/.png/.webp -> .pdf, .pdf -> per-page .png/.jpg)
- [ ] Image Resizer / Batch Resizer (.png, .jpg, .webp, .bmp)
- [ ] Font Converter (.ttf, .otf -> .woff, .woff2, and vice versa)
- [ ] QR Code Generator (text, URL, vCard -> .png, .svg)
- [ ] Hash Generator (any file -> MD5, SHA-1, SHA-256, SHA-512 output)
- [ ] File Splitter / Joiner (any large file -> split into N parts, rejoin from parts)

## Debugging

### Text Editor Bugs

- [ ] Cannot highlight and scroll at the same time? - Hard
- [ ] Undoing all changes does not remove "modified" file information status
- [ ] Last letter of text when highlighting appears white, while the rest of the text is black while in dark mode

### Image Editor Bugs

- [ ] Brush, fill, etc. sometimes select image instead of applying
- [ ] Zooming in and out while moving image/text desyncs it from the cursor

### Json Editor Bugs

- [ ] Sort and Search do not work on text view
- [ ] Undoing all changes does not remove "modified" file information status

### Document Editor Bugs

- [ ] Doesn't load horizontal lines properly
- [ ] Highlighting
  - [ ] Cannot highlight and scroll at the same time
  - [ ] Highlighting seems to not align perfectly with letters, especially with large font sizes
- [ ] Page Break
  - [ ] Changing font size, font type, or page margins causes paragraphs to disconnect sometimes
  - [ ] Sometimes page breaks cut a paragraph in the middle of a word
- [ ] Tab/Indent
  - [ ] Undoing a tab/indent moves cursor one character too far
  - [ ] Cannot tab/indent with multi-line highlights
  - [ ] Cannot tab/indent multiple lines at once, removes the lines instead

### Other Bugs

- [ ] Save warning dialogue does not show up on application close

## Features

### Text Editor

- [ ] Latex?
- [ ] Spell Check
- [ ] Grammar Check
- [ ] Hovering over links with ctrl pressed should change the cursor to a pointer
- [ ] Hovering over a checklist item should change cursor to a pointer

### Image Editor

- [ ] Selection tools
  - [ ] Ellipse
  - [ ] Free-hand lasso
  - [ ] Magic wand
  - [ ] Copy/paste/cut within selection
- [ ] Gradient tool
  - [ ] Linear
  - [ ] Radial
- [ ] Cutout tool
- [ ] Improve realism of canvas texture type
- [ ] Shape tool
- [ ] Pattern/texture fill
- [ ] Perspective/affine warp
- [ ] Edge detection filter
- [ ] Snap to grid for text/crop

### Json Editor

- [ ] Make the sorting make a little more sense
- [ ] Add key popup needs a little more information
- [ ] Search should only show up when clicking ctrl+f
- [ ] Make navigation bigger and more noticeable
- [ ] Add true "Table" view, should look like an excel table for example
- [ ] Center Buttons in Popup Modals (E.g New file, Add key)
- [ ] Add loading screen for sorting and changing views

### Document Editor

- [ ] Spell check
- [ ] Grammar Check
- [ ] Document Templates
- [ ] Add way to create comments
- [ ] Increase indent and decrease indent buttons
- [ ] Different font's per line/text instead of changing the whole file to one font
- [ ] Checklists
- [ ] Tables
- [ ] Images
- [ ] Links
- [ ] Highlighting
- [ ] Ability to change look of headers/normal text
- [ ] Printing
- [ ] Header and footers
- [ ] Clean up toolbar
- [ ] Shift tab should remove indent

### CONVERTERS

- [ ] Keyboard Shortcuts

## Other

## Testing

- [ ] Create Testing Framework
