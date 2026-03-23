# UniEdit Todo

## General

- [ ] Create Main Menu
  - [ ] Add way to show more than 3 converters and screen (either scrollbar or list vertically)
- [ ] Improve look of patch notes and settings modal
- [ ] Editable keyboard shortcuts per page

## Screens

- [ ] Code Editor (.rs, .py, .js, .ts, .c, .cpp, .go, .java, .sh, .css, .sql)
- [ ] Spreadsheet Editor (.csv, .tsv, .xlsx, .xls, .ods)
- [ ] Document Editor (.docx, .doc, .odt, .pdf)
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
- [ ] Archive Converter (.zip, .tar.gz, .7z, .bz2)
- [ ] Font Converter (.ttf, .otf -> .woff, .woff2, and vice versa)
- [ ] QR Code Generator (text, URL, vCard -> .png, .svg)
- [ ] Hash Generator (any file -> MD5, SHA-1, SHA-256, SHA-512 output)
- [ ] File Splitter / Joiner (any large file -> split into N parts, rejoin from parts)

## Debugging

### Text Editor Bugs

- [ ] Cannot highlight and scroll at the same time? - Hard
- [ ] Undoing all changes does not remove "modified" file information status

### Image Editor Bugs

- [ ] Eyedrop tool lags on very large images
- [ ] On very large images, there is a slight lag at the end of brush strokes, eraser strokes, or retouch tool strokes
- [ ] Layer drawings and edits stay on image even after being cropped using crop tool
- [ ] Pan tool doesn't allow for left click panning
- [ ] Vibrance retouch tool has a red outline
- [ ] Blur and Sharpen tool laggy on very large images when moving mouse fast
- [ ] Image wide filters do not work on image layers (Greyscale, invert, and sepia do however)
- [ ] Image layer CAN merge downwards onto a rasterized layer using ctrl+e or the top toolbar, but not using the layer sidebar
- [ ] Cannot use image top toolbar transformation to rotate an image layer
- [ ] Rasterizing image layer causes it to be invisible (it still exists, just cant be seen, the second any edits are made it is visible)

### Json Editor Bugs

- [ ] Sort and Search do not work on text view
- [ ] Undoing all changes does not remove "modified" file information status

### Other Bugs

- [ ] Save warning dialogue does not show up on application close
- [ ] Some pages have a double separator line within the "View" tab on the top toolbar

## Features

### Text Editor

- [ ] Latex?
- [ ] Spell Check
- [ ] Grammar Check

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
- [ ] Add way to save layers and settings per image

### Json Editor

- [ ] Make the sorting make a little more sense
- [ ] Add key popup needs a little more information
- [ ] Search should only show up when clicking ctrl+f
- [ ] Change cursor when hovering over buttons/navigation
  - [ ] Make navigation bigger and more noticeable
- [ ] Add true "Table" view, should look like an excel table for example
- [ ] Center Buttons in Popup Modals (E.g New file, Add key)
- [ ] Add loading screen for sorting and changing views

### CONVERTER: Img 2 Img

- [ ] Keyboard Shortcuts

## Other

## Testing

- [ ] Create Testing Framework
