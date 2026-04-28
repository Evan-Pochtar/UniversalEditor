# Universal Editor Patch Notes

## V0.0.1

#### Features

- **IMG2IMG CONVERTER**: Img2Img Converter added (jpeg, png, webp, bmp, tiff, ico file options)
- **TEXT EDITOR**: Header Level 4 on Markdown Mode (E.g ####)
- **TEXT EDITOR**: Adding warning dialogue for leaving an unsaved file.
- **GENERAL**: Adding Logo

#### Debugging

- **TEXT EDITOR**: Codeblock Background now sync's properly (mostly)
- **TEXT EDITOR**: Cursor now no longer desyncs when multi-line codeblocks are present
- **TEXT EDITOR**: Cursor now no longer desyncs on load with old files
- **GENERAL**: Sidebar now no longer shows empty white box instead of down arrow

## V0.0.2

#### Features

- **IMG EDITOR**: First prototype of image editor
- **IMG EDITOR**: Exporting to other image types now allowed (jpeg, png, webp, bmp, tiff, ico)
- **IMG EDITOR**: Option for preservation of metadata
- **IMG EDITOR**: Loading screen for applying filters
- **IMG EDITOR**: Color picker now supports hex codes
- **IMG2IMG CONVERTER**: Error's in image processing now displayed on UI
- **IMG2IMG CONVERTER**: .ico now has auto-scale to 256px option
- **TEXT EDITOR**: Saved Toolbar/File info settings between sessions
- **TEXT EDITOR**: Tooltips for Headers include Keyboard Shortcut
- **TEXT EDITOR**: Automatic view mode detection based off of file extension
- **GENERAL**: Added "System" Theme
- **GENERAL**: Theme setting saves between sessions
- **GENERAL**: Now able to remove files from "recent files" list on sidebar
- **GENERAL**: Adding trashcan icon instead of "x" for recent files removal

#### Debugging

- **GENERAL**: Removed Visibility of Toolbar/File info settings when not in text editor
- **GENERAL**: Images now show up in recent files when opened from main menu

## V0.0.3

#### Features

- **IMG EDITOR**: Added Keyboard Shortcuts for various tools in image editor
- **IMG EDITOR**: Now allows for image stretching/expanding with resizing.
- **IMG EDITOR**: Corner resizing and selection of text now available
- **IMG EDITOR**: Textbox up/down resizing
- **IMG EDITOR**: Textbox Bold/Italics/Underlining
- **IMG EDITOR**: Textbox Rotating
- **IMG EDITOR**: Roboto and Ubuntu fonts now available within text tool
- **GENERAL**: Top panel now customized to each page
- **GENERAL**: Adding icon to release build executable
- **GENERAL**: Improved Home Page
- **GENERAL**: Can now navigate to home page from any screen or converter
- **GENERAL**: Improved Settings Menu and Patch notes Parsing
- **GENERAL**: Modals now close when clicked off from instead of just using the "x"

#### Debugging

- **IMG EDITOR**: Fixed issue only allowing for drawing while dragging
- **IMG EDITOR**: Images now show up in "Recent Files" properly
- **IMG EDITOR**: Text tool now properly shows up in final images
- **IMG EDITOR**: Bold text now shows up in edited canvas, not only in final image
- **IMG EDITOR**: Rotated text now displays properly on export
- **IMG EDITOR**: Rotated text no longer desyncs from canvas position
- **IMG EDITOR**: Can now properly highlight text in text boxes
- **IMG EDITOR**: Selecting another tool now properly deselects active textbox
- **IMG EDITOR**: Sentences now wrap within text boxes
- **IMG EDITOR**: Text boxes now use selected font properly
- **IMG EDITOR**: Cursor no longer gets disconnected from text in text box

## V0.0.4

#### Features

- **IMG EDITOR**: Filters and Transformations moved to top menu bar under "Image" and "Filter"
- **IMG EDITOR**: Eraser now has option to remove background
- **IMG EDITOR**: Crop is now readjustable and displays width and height of crop
- **IMG EDITOR**: Cursor now changes depending on current tool selected
- **IMG EDITOR**: Filter panel is now it's own modal similar to the color picker
- **IMG EDITOR**: Brush presets now available (Regular, Pen, Pencil, Crayon, Marker, Calligraphy Pen, Spray, Watercolor, Charcoal, Airbrush)
- **IMG EDITOR**: Brush shapes now available (Circle, Square, Diamond, Flat (Calligraphy style), Star)
- **IMG EDITOR**: Brush parameters are now changeable
- **IMG EDITOR**: Saving custom brushes as favorites now available
- **IMG EDITOR**: Brush Textures of different types (Rough, Paper, Canvas)
- **IMG EDITOR**: Favorites added to the color picker (with keyboard shortcuts)
- **IMG EDITOR**: Retouch tool added (Blur, Sharpen, Smudge, Vibrance, Saturation, Temperature, and Brightness)
- **IMG EDITOR**: Added pixelate to retouch tool
- **IMG EDITOR**: Added Gradient sliders for Hue, Brightness, Temperature, and Vibrance on both Filter Panel and Retouch tool
- **IMG EDITOR**: Added Filter panel preview
- **TEXT EDITOR**: Now uses Ubuntu and Roboto fonts instead of Mono and Sans
- **TEXT EDITOR**: Default font size and font type can now be selected in settings
- **TEXT EDITOR**: Adding functionality for checkbox lists
- **TEXT EDITOR**: Adding functionality for block quotes
- **TEXT EDITOR**: Adding functionality for bold and italic text together

#### Debugging

- **IMG EDITOR**: Eraser now properly erases to white background
- **IMG EDITOR**: Cursor now no longer disappears on color picker
- **IMG EDITOR**: Cursor now no longer disappears on unsaved changes screen
- **IMG EDITOR**: Crop tool now properly resizes
- **IMG EDITOR**: Brushes, eraser, and retouch filters now no longer lag on large images
- **IMG EDITOR**: Sharpen tool no longer immediately causes artifacting
- **IMG EDITOR**: Zoom, Aspect Ratio, and Color Picker no longer are shown in Retouch tool
- **GENERAL**: Console no longer opens on build executable
- **GENERAL**: Menu version now displays cargo package version
- **GENERAL**: Can no longer click on buttons in the background with the patch notes or settings modal up

## V0.0.5

#### Features

- **JSON EDITOR**: First prototype of the Json Editor
- **JSON EDITOR**: Undo and Redo moved to top bar
- **JSON EDITOR**: Improved styling and cleaned up backgrounds
- **JSON EDITOR**: Added option to hide file info in both settings and "View" toolbar
- **JSON EDITOR**: Search bar can now search non expanded nodes
- **IMG EDITOR**: Star shape removed from brush settings
- **TEXT EDITOR**: Can now change between txt and md using file information context window (right click)
- **TEXT EDITOR**: Can now rename file in the file information context window (right click)
- **TEXT EDITOR**: Can now open file location using file information context widow (right click)
- **TEXT EDITOR**: Added the ability to load tables in the markdown mode
- **DATA CONVERTER**: First prototype of the data converter
- **GENERAL**: Added support for outputting AVIF files for the Image Editor and Image Converter
- **GENERAL**: Added easier way to add new screens/converters to the app (Code Only Change)
- **GENERAL**: Improved Light mode color scheme to be more pleasing to the eye

#### Debugging

- **JSON EDITOR**: Fixed crashing when scrolling down on large JSON's
- **JSON EDITOR**: Search now properly navigates to searched keys/values
- **JSON EDITOR**: Error popup text no longer leaves the popup
- **JSON EDITOR**: JSON Editor no longer crashes when trying to navigate 2 or more parents back
- **JSON EDITOR**: Can now undo/redo on the text view properly
- **JSON EDITOR**: No longer converts new boolean values to strings
- **IMG EDITOR**: Options bar buttons, sliders, and drag values are now centered
- **IMG EDITOR**: Preview filter button now can be turned off and on, rather than having to click cancel
- **IMG EDITOR**: Sliders and button text are now visible within light mode, slightly more hover visibility with buttons in dark mode
- **IMG EDITOR**: Fixed backgrounds and text color of some buttons within "Brush Settings" in light mode
- **IMG EDITOR**: Text box now rotates correctly on flip vertical and flip horizontal
- **IMG EDITOR**: Text box now correctly exports with wrapped words
- **TEXT EDITOR**: Code blocks and Blockquotes no longer desync on font change

## V0.0.6

#### Features

- **GENERAL**: Added right click context menu in "Recent Files" section of the sidebar
- **GENERAL**: Allow for right click, open with universal_editor on windows
- **JSON EDITOR**: Added Line numbers to the text view
- **IMG EDITOR**: Added Layer System to the Image Editor
- **IMG EDITOR**: Color picker panel information now centered
- **IMG EDITOR**: Increased size of selection sliders
- **IMG EDITOR**: Adjusted Spray brush preset to be thicker
- **IMG EDITOR**: Adjusted Marker to display with higher default wetness
- **IMG EDITOR**: Improved performance of retouch tool filters
- **IMG EDITOR**: Custom Slider for Saturation in the retouch tool
- **IMG EDITOR**: Added Image Imports and Image Layers
- **IMG EDITOR**: Renamed "Pan" tool to "Select/Pan" tool

#### Debugging

- **TEXT EDITOR**: Bolding formatting no longer makes divider line before typing
- **JSON EDITOR**: Fixed lag on large sized files within text view
- **JSON EDITOR**: Scroll "bounce" issue with large files on small display screens fixed
- **JSON EDITOR**: No longer pre-formats JSON and uses it's raw data instead
- **JSON EDITOR**: Ctrl+S save shortcut now works as intended
- **JSON EDITOR**: "Modified" and "Saved" file information status now shows up correctly
- **JSON EDITOR**: Long numbers no longer get replaced by scientific numbers
- **IMG EDITOR**: Eraser now properly erases on layers
- **IMG EDITOR**: Opacity slider now only loads when you are no longer sliding to prevent lag
- **IMG EDITOR**: Brush now no longer periodically erases background while drawing
- **IMG EDITOR**: No longer has jumpy cursor on the bottom of the color picker square
- **IMG EDITOR**: Hue slider no longer sometimes moves color square cursor
- **IMG EDITOR**: Can now scroll down on the color picker panel
- **IMG EDITOR**: Textbox no longer lags on large images
- **IMG EDITOR**: Textbox no longer allows empty textbox's (deletes layer as well)
- **IMG EDITOR**: Stamped text now matches the exact size of preview text
- **IMG EDITOR**: Background layer brush strokes no longer sometimes remove background of layer brush strokes
- **IMG EDITOR**: Drag value input no longer interferes with color picker favorites
- **IMG EDITOR**: Imported images now work properly with the layer system
- **IMG EDITOR**: Text and imported images now can be used with the "Select/pan" tool
- **IMG EDITOR**: Moving imported image no longer moves background canvas
- **IMG EDITOR**: No longer allows for both imported image AND textbox to be selected at once

## V0.0.7

#### Features

- **IMG EDITOR**: Improved quality of edited and exported imported images/text
- **IMG EDITOR**: "Place Image" renamed to "Import to Canvas" and moved to top bar under "File"
- **IMG EDITOR**: Improved paper and rough texture in the brush settings
- **IMG EDITOR**: Favorite custom brushes now have keyboard shortcuts similar to favorite color (Ctrl+1-9, Ctrl+0)
- **IMG EDITOR**: Custom brushes can now be exported and imported
- **IMG EDITOR**: Added a brush preview to brush settings
- **IMG EDITOR**: Can now save layer and brush settings per image, with option to delete in main menu
- **IMG EDITOR**: Added button to rasterize text box into rasterized layer
- **TEXT EDITOR**: Removing constantly updating character count, moved to top toolbar under "File", option named "Word Count"
- **GENERAL**: Added "About" modal in the main menu to describe the current state of the project
- **GENERAL**: Added keyboard shortcut to close sidebar (Ctrl+Backslash)
- **GENERAL**: Sidebar "Recent Files" list now differentiates similarly named files that are located in different folders
- **GENERAL**: Removed FPS counter to improve performance

#### Debugging

- **JSON EDITOR**: Newlines no longer display on string preview in the tree view
- **IMG EDITOR**: Jpeg and JPG's now load the same into the canvas
- **IMG EDITOR**: "Fit" now properly fits the screen to small canvases
- **IMG EDITOR**: Can now select and see the selection bounds of textbox's even when hiding beneath an image layer
- **IMG EDITOR**: In a rasterized layer, the brush no longer fades to dark, and fades depending on the background instead
- **IMG EDITOR**: Brush, eraser, and retouch no longer lag on large images while in rasterized layers
- **IMG EDITOR**: Can no longer move the background layer
- **IMG EDITOR**: Rasterized images are no longer invisible
- **IMG EDITOR**: Cropped layer drawings are no longer included in the final image
- **IMG EDITOR**: Pan tool now properly allows for panning across the image when not selected
- **IMG EDITOR**: Vibrance calculation now no longer sometimes adds a red "hue" or outline
- **IMG EDITOR**: Image-Wide top bar filters now work on imported images
- **IMG EDITOR**: Eyedrop tool now has the same performance no matter the image size
- **IMG EDITOR**: Image layers can now be merged with rasterized layers
- **IMG EDITOR**: Can now use the top toolbar to rotate images and text
- **IMG EDITOR**: Performance improvements with sharpen and blur retouch tools on large images
- **IMG EDITOR**: On large images, there is no longer a slight lag at the end of brush strokes, eraser strokes, or retouch strokes
- **IMG EDITOR**: Can now use the eyedropper on imported image layers
- **IMG EDITOR**: Textbox no longer becomes invisible after merging into a rasterized or background layer
- **IMG2IMG CONVERTER**: JPG's now load properly into the input space
- **GENERAL**: File names no longer go outside of sidebar buttons bounds
- **GENERAL**: In Converters with no view items, as well as the main menu, there are no longer double separators

## V0.0.8

#### Features

- **ARCHIVE CONVERTER**: First prototype of the archive converter
- **ARCHIVE CONVERTER**: Now works with .7z files for both converting and input
- **DOCUMENT EDITOR**: First prototype of the document editor
- **DOCUMENT EDITOR**: Font size now works similarly to other document editor software, base font size is now located within page settings
- **DOCUMENT EDITOR**: Adding more page presets and changing margin measurement to inches instead of pixels
- **DOCUMENT EDITOR**: Added Horizontal Rule
- **DOCUMENT EDITOR**: Added ability to change color of text
- **DOCUMENT EDITOR**: Added numbered and non-numbered lists
- **DOCUMENT EDITOR**: Added document editor to the about modal within the main menu
- **DOCUMENT EDITOR**: Document editor now supports .odt files
- **JSON EDITOR**: Added hover cursor effects
- **JSON EDITOR**: Added File Renaming, Opening file in converter, and Open File Location by right clicking file name in file information bar
- **TEXT EDITOR**: Added hover cursor effects
- **GENERAL**: Improved look of settings and patch notes modal
- **GENERAL**: New way to show more than 3 screens and converters on main menu (for future additions)
- **GENERAL**: Adding two new fonts, Open Sans and Google Sans

#### Debugging

- **TEXT EDITOR**: Last character of lines/paragraphs is no longer a different color when highlighting (dark mode)
- **DOCUMENT EDITOR**: No longer creates non-editable newlines between text when loading documents
- **DOCUMENT EDITOR**: Backspace now no longer removes the cursor on empty documents
- **DOCUMENT EDITOR**: No longer puts very long lines with over a page of text on separate pages
- **DOCUMENT EDITOR**: Fixed both text highlighting and multi-line highlighting appearing at the same time
- **DOCUMENT EDITOR**: Can now undo after pasting text
- **DOCUMENT EDITOR**: Multi-line highlight now works on all pages, not just the first few
- **DOCUMENT EDITOR**: Multi-line highlighting now no longer makes vertical selections while moving mouse diagonally.
- **DOCUMENT EDITOR**: Now loads bullet points properly for both docx and odt files
- **DOCUMENT EDITOR**: Tabbing at the start of a paragraph no longer indents the whole paragraph, only the first line
- **DOCUMENT EDITOR**: Multi-line Highlighting and in line highlighting are now the same

## V0.0.9

#### Features
- **DOCUMENT EDITOR**: Font now works per character not for the whole document at once
- **DOCUMENT EDITOR**: Added the ability to create links
- **DOCUMENT EDITOR**: Added the ability to highlight text
- **DOCUMENT EDITOR**: Added the ability to create checklists
- **DOCUMENT EDITOR**: Undo stack improved to undo changes a little better while typing
- **DOCUMENT EDITOR**: Headers now change font size and styling rather than hard coded size increases
- **DOCUMENT EDITOR**: Added increase/decrease paragraph indent buttons (will be moved to top toolbar in the future)

#### Debugging
- **DOCUMENT EDITOR**: Up and down arrows now properly navigate through pages
- **DOCUMENT EDITOR**: Highlights now load from both .odt and .docx files
- **DOCUMENT EDITOR**: Font's now transfer from exported docx files
- **DOCUMENT EDITOR**: Links now properly load from both .odt and .docx files
- **DOCUMENT EDITOR**: Subscript no longer loads as superscript
- **DOCUMENT EDITOR**: Fixed issues relating to placing tab indents and loading tab indents
- **DOCUMENT EDITOR**: Headers, Titles, Subtitles, etc. now export correctly
- **DOCUMENT EDITOR**: Tab indents and paragraph indents now the same size
- **DOCUMENT EDITOR**: Undoing tabs no longer moves the cursor one space too far
