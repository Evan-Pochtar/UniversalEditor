# UniEdit Todo

## General

- [ ] Create Main Menu
  - [ ] Add About Page
- [ ] Keyboard shortcuts to close/open sidebar, toolbar, etc.
- [ ] Add right click context menu to change file name and open file location in "recent files" list

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
- [ ] Data Format Converter (.json, .yaml, .toml, .xml, .csv)
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

### Image Editor Bugs

- [ ] Star brush type doesn't really look star like
- [ ] Preview button should be able to be turned off and on, rather than having to click cancel
- [ ] Some text on buttons is white in light mode
- [ ] Hover effects of buttons only work in light mode
- [ ] Sliders are only visible in dark mode
- [ ] Text Issues
  - [ ] Exported Image doesn't fully contain wrapped text if there are no spaces (E.g one really long word that spans multiple lines)
  - [ ] Vertical and Horizontal Rotations do not rotate textbox properly

### Json Editor Bugs

- [ ] Sort and Search do not work on text view

### Other Bugs

- [ ] Save warning dialogue does not show up on application close

## Features

### Text Editor

- [ ] Ability to load tables in markdown mode
- [ ] Latex?

### Image Editor

- [ ] Layer system
  - [ ] Add
  - [ ] Reorder
  - [ ] Merge
  - [ ] Opacity
- [ ] Selection tools
  - [ ] Ellipse
  - [ ] Free-hand lasso
  - [ ] Magic wand
  - [ ] Copy/paste/cut within selection
- [ ] Gradient tool
  - [ ] Linear
  - [ ] Radial
- [ ] Cutout tool
- [ ] Retouch tool
  - [ ] Saturation tool should have a custom slider
  - [ ] Improve performance on bigger sizes
- [ ] Shape tool
- [ ] Brush Settings Panel
  - [ ] Improve Presets to be more "realistic"
  - [ ] Keyboard shortcuts to change between presets/favorite brushes
  - [ ] Export custom brushes
  - [ ] Improve realism of canvas and paper texture type
  - [ ] Hover tooltips for more information about parameters
  - [ ] Brush preview
  - [ ] Library of custom brushes?
- [ ] Import Images into Canvas
  - [ ] Drag-and-drop image open
- [ ] Pattern/texture fill
- [ ] Perspective/affine warp
- [ ] Edge detection filter
- [ ] Snap to grid for text/crop

### Json Editor
- [ ] Make the sorting make a little more sense
- [ ] Add key popup needs a little more information
- [ ] Need to add more space for text in very long key values
- [ ] Search should only show up when clicking ctrl+f
- [ ] Change cursor when hovering over buttons/navigation
  - [ ] Make navigation bigger and more noticeable
- [ ] Add true "Table" view, should look like an excel table for example
- [ ] Option to increase size of each row in the tree view
- [ ] Add line numbers to text view
- [ ] Center Buttons in Popup Modals (E.g New file, Add key)

### CONVERTER: Img 2 Img

- [ ] Keyboard Shortcuts

## Other

## Testing

- [ ] Create Testing Framework
