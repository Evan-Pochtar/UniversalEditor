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

- [ ] Undoing all changes does not remove "modified" file information status
- [ ] Highlighting multiple markdown items (especially tables) shows both the plaintext and the markdown rendering at the same time

### Image Editor Bugs

- [ ] Brush, fill, etc. sometimes select image instead of applying
- [ ] Zooming in and out while moving image/text desyncs it from the cursor

### Json Editor Bugs

- [ ] Sort and Search do not work on text view
- [ ] Undoing all changes does not remove "modified" file information status

### Document Editor Bugs

- [ ] Navigating up beyond a page break sometimes brings the cursor one line too far up
- [ ] Cursor gets stuck when making text larger than 1 page with font size changes
- [ ] Sometimes cuts off last line of text below loaded image
- [ ] When increasing font size in table cell text, it does not expand the size of the cell
- [ ] Can't right click to open context menu while editing text in table cell
- [ ] Copy with formatting doesn't work with multi-line highlights
- [ ] ODT Issues
  - [ ] Doesn't load horizontal lines properly (.odt)
  - [ ] Doesn't load checklists properly (.odt)
  - [ ] Superscript and subscript dont load (.odt)
  - [ ] Double indents sometimes disappear on load (.odt)
  - [ ] Empty spaces (newlines) are deleted on load (.odt)
  - [ ] ODT Exporting has many major flaws

### Other Bugs

- [ ] Save warning dialogue does not show up on application close

## Features

### Text Editor

- [ ] Latex?
- [ ] Spell Check
- [ ] Grammar Check
- [ ] Images
- [ ] Footnotes
- [ ] Definition list
- [ ] Emojis
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

- [ ] Add key popup needs a little more information
- [ ] Make navigation bigger and more noticeable
- [ ] Add true "Table" view, should look like an excel table for example
- [ ] Center Buttons in Popup Modals (E.g New file, Add key)
- [ ] Add loading screen for sorting and changing views

### Document Editor

- [ ] Spell check
- [ ] Grammar Check
- [ ] Document Templates
- [ ] Add way to create comments
- [ ] Export
  - [ ] PDF
  - [ ] MD
  - [ ] rtf
  - [ ] epub
  - [ ] html
- [ ] Ability to change look of headers/normal text
- [ ] Printing
- [ ] Page Breaks
- [ ] Header and footers
- [ ] Cell colors in tables for dark mode
- [ ] Increase max table size creation from the table button

### CONVERTERS

- [ ] Keyboard Shortcuts

## Other

## Testing

- [ ] Create Testing Framework
