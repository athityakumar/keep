use std::path::Path;

use ansi_term::{ANSIString, Style};

use fs::{File, FileTarget};
use output::escape;
use output::cell::TextCellContents;
use output::render::FiletypeColours;

// use std::io::prelude::*;
// use std::env;
// extern crate yaml_rust;
// use self::yaml_rust::{YamlLoader, YamlEmitter};

/// Basically a file name factory.
#[derive(Debug)]
pub struct FileStyle {

    /// Whether to append file class characters to file names.
    pub classify: Classify,

    /// Mapping of file extensions to colours, to highlight regular files.
    pub exts: Box<FileColours>,
}

impl FileStyle {

    /// Create a new `FileName` that prints the given file’s name, painting it
    /// with the remaining arguments.
    pub fn for_file<'a, 'dir, C: Colours>(&'a self, file: &'a File<'dir>, colours: &'a C) -> FileName<'a, 'dir, C> {

        FileName {
            file, colours,
            link_style: LinkStyle::JustFilenames,
            classify:   self.classify,
            exts:       &*self.exts,
            target:     if file.is_link() { Some(file.link_target()) }
                                     else { None }
        }
    }
}


/// When displaying a file name, there needs to be some way to handle broken
/// links, depending on how long the resulting Cell can be.
#[derive(PartialEq, Debug, Copy, Clone)]
enum LinkStyle {

    /// Just display the file names, but colour them differently if they’re
    /// a broken link or can’t be followed.
    JustFilenames,

    /// Display all files in their usual style, but follow each link with an
    /// arrow pointing to their path, colouring the path differently if it’s
    /// a broken link, and doing nothing if it can’t be followed.
    FullLinkPaths,
}


/// Whether to append file class characters to the file names.
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Classify {

    /// Just display the file names, without any characters.
    JustFilenames,

    /// Add a character after the file name depending on what class of file
    /// it is.
    AddFileIndicators,
}

impl Default for Classify {
    fn default() -> Classify {
        Classify::JustFilenames
    }
}



/// A **file name** holds all the information necessary to display the name
/// of the given file. This is used in all of the views.
pub struct FileName<'a,  'dir: 'a,  C: Colours+'a> {

    /// A reference to the file that we’re getting the name of.
    file: &'a File<'dir>,

    /// The colours used to paint the file name and its surrounding text.
    colours: &'a C,

    /// The file that this file points to if it’s a link.
    target: Option<FileTarget<'dir>>,

    /// How to handle displaying links.
    link_style: LinkStyle,

    /// Whether to append file class characters to file names.
    classify: Classify,

    /// Mapping of file extensions to colours, to highlight regular files.
    exts: &'a FileColours,
}


impl<'a, 'dir, C: Colours> FileName<'a, 'dir, C> {

    /// Sets the flag on this file name to display link targets with an
    /// arrow followed by their path.
    pub fn with_link_paths(mut self) -> Self {
        self.link_style = LinkStyle::FullLinkPaths;
        self
    }

    /// Paints the name of the file using the colours, resulting in a vector
    /// of coloured cells that can be printed to the terminal.
    ///
    /// This method returns some `TextCellContents`, rather than a `TextCell`,
    /// because for the last cell in a table, it doesn’t need to have its
    /// width calculated.
    pub fn paint(&self) -> TextCellContents {
        let mut bits = Vec::new();

        if self.file.parent_dir.is_none() {
            if let Some(parent) = self.file.path.parent() {
                self.add_parent_bits(&mut bits, parent);
            }
        }

        if !self.file.name.is_empty() {
        	// The “missing file” colour seems like it should be used here,
        	// but it’s not! In a grid view, where there's no space to display
        	// link targets, the filename has to have a different style to
        	// indicate this fact. But when showing targets, we can just
        	// colour the path instead (see below), and leave the broken
        	// link’s filename as the link colour.
            for bit in self.coloured_file_name() {
                bits.push(bit);
            }
        }

        if let (LinkStyle::FullLinkPaths, Some(target)) = (self.link_style, self.target.as_ref()) {
            match *target {
                FileTarget::Ok(ref target) => {
                    bits.push(Style::default().paint(" "));
                    bits.push(self.colours.normal_arrow().paint("->"));
                    bits.push(Style::default().paint(" "));

                    if let Some(parent) = target.path.parent() {
                        self.add_parent_bits(&mut bits, parent);
                    }

                    if !target.name.is_empty() {
                        let target = FileName {
                            file: target,
                            colours: self.colours,
                            target: None,
                            link_style: LinkStyle::FullLinkPaths,
                            classify: Classify::JustFilenames,
                            exts: self.exts,
                        };

                        for bit in target.coloured_file_name() {
                            bits.push(bit);
                        }
                    }
                },

                FileTarget::Broken(ref broken_path) => {
                    bits.push(Style::default().paint(" "));
                    bits.push(self.colours.broken_symlink().paint("->"));
                    bits.push(Style::default().paint(" "));
                    escape(broken_path.display().to_string(), &mut bits, self.colours.broken_filename(), self.colours.broken_control_char());
                },

                FileTarget::Err(_) => {
                    // Do nothing -- the error gets displayed on the next line
                },
            }
        }
        else if let Classify::AddFileIndicators = self.classify {
            if let Some(class) = self.classify_char() {
                bits.push(Style::default().paint(class));
            }
        }

        bits.into()
    }


    /// Adds the bits of the parent path to the given bits vector.
    /// The path gets its characters escaped based on the colours.
    fn add_parent_bits(&self, bits: &mut Vec<ANSIString>, parent: &Path) {
        let coconut = parent.components().count();

        if coconut == 1 && parent.has_root() {
            bits.push(self.colours.symlink_path().paint("/"));
        }
        else if coconut >= 1 {
            escape(parent.to_string_lossy().to_string(), bits, self.colours.symlink_path(), self.colours.control_char());
            bits.push(self.colours.symlink_path().paint("/"));
        }
    }


    /// The character to be displayed after a file when classifying is on, if
    /// the file’s type has one associated with it.
    fn classify_char(&self) -> Option<&'static str> {
        if self.file.is_executable_file() {
            Some("*")
        } else if self.file.is_directory() {
            Some("/")
        } else if self.file.is_pipe() {
            Some("|")
        } else if self.file.is_link() {
            Some("@")
        } else if self.file.is_socket() {
            Some("=")
        } else {
            None
        }
    }


    /// Returns at least one ANSI-highlighted string representing this file’s
    /// name using the given set of colours.
    ///
    /// Ordinarily, this will be just one string: the file’s complete name,
    /// coloured according to its file type. If the name contains control
    /// characters such as newlines or escapes, though, we can’t just print them
    /// to the screen directly, because then there’ll be newlines in weird places.
    ///
    /// So in that situation, those characters will be escaped and highlighted in
    /// a different colour.
    fn coloured_file_name<'unused>(&self) -> Vec<ANSIString<'unused>> {
        let file_style = self.style();
        let mut bits = Vec::new();

        // let mut fs = File::open("/Users/athityakumar/Documents/GitHub/athityakumar/colorls/lib/yaml/files.yaml");

        // let docs = YamlLoader::load_from_str(fs).unwrap();
        // let doc = &docs[0];

        let words = self.file.name.clone();
        let words: Vec<&str> = words.split(".").collect();
        let format = words.last();
        // let words_size = words.len();
        // println!("{:?}", words.last());

        // doc["foo"][0].as_str().unwrap()

        let icon;

        if self.file.is_directory() {
            icon = "\u{f115}  ".to_string();            
        } else if [Some(&"zip"), Some(&"tar"), Some(&"gz"), Some(&"rar")].contains(&format) {
            icon = "\u{f410}  ".to_string();
        } else if [Some(&"yml"), Some(&"yaml")].contains(&format) {
            icon = "\u{f481}  ".to_string();
        } else if [Some(&"xml"), Some(&"xul")].contains(&format) {
            icon = "\u{e619}  ".to_string();
        } else if [Some(&"xls"), Some(&"xlsx"), Some(&"csv"), Some(&"gsheet")].contains(&format) {
            icon = "\u{f17a}  ".to_string();
        } else if [Some(&"ini"), Some(&"exe"), Some(&"bat")].contains(&format) {
            icon = "\u{f481}  ".to_string();
        } else if [Some(&"webm"), Some(&"ogv"), Some(&"mp4"), Some(&"mkv"), Some(&"avi")].contains(&format) {
            icon = "\u{f03d}  ".to_string();
        } else if [Some(&"styl"), Some(&"stylus"), Some(&"tex")].contains(&format) {
            icon = "\u{e600}  ".to_string();
        } else if [Some(&"zshrc"), Some(&"zsh-theme"), Some(&"zsh"), Some(&"sh"), Some(&"fish"), Some(&"bashrc"), Some(&"bash_profile"), Some(&"bash_history"), Some(&"bash")].contains(&format) {
            icon = "\u{f489}  ".to_string();
        } else if [Some(&"rb"), Some(&"ru"), Some(&"rspec"), Some(&"rspec_status"), Some(&"rspec_parallel"), Some(&"Rakefile"), Some(&"Procfile"), Some(&"lock"), Some(&"gemspec"), Some(&"Gemfile"), Some(&"Guardfile")].contains(&format) {
            icon = "\u{e21e}  ".to_string();
        } else if [Some(&"bmp"), Some(&"gif"), Some(&"ico"), Some(&"png"), Some(&"jpg"), Some(&"jpeg"), Some(&"svg")].contains(&format) {
            icon = "\u{f1c5}  ".to_string();
        } else if [Some(&"eot"), Some(&"otf"), Some(&"ttf"), Some(&"woff"), Some(&"woff2")].contains(&format) {
            icon = "\u{f031}  ".to_string();
        } else if [Some(&"md"), Some(&"txt"), Some(&"rst"), Some(&"rdoc")].contains(&format) {
            icon = "\u{f48a}  ".to_string();
        } else if [Some(&"erb"), Some(&"slim")].contains(&format) {
            icon = "\u{e73b}  ".to_string();
        } else if [Some(&"RData"), Some(&"rds"), Some(&"r")].contains(&format) {
            icon = "\u{f25d}  ".to_string();
        } else if [Some(&"py"), Some(&"pyc")].contains(&format) {
            icon = "\u{e606}  ".to_string();
        } else if [Some(&"ppt"), Some(&"pptx"), Some(&"gslides")].contains(&format) {
            icon = "\u{f1c4}  ".to_string();
        } else if [Some(&"git"), Some(&"gitignore"), Some(&"gitconfig"), Some(&"gitignore_global")].contains(&format) {
            icon = "\u{f1d3}  ".to_string();
        } else if [Some(&"apk"), Some(&"gradle")].contains(&format) {
            icon = "\u{e70e}  ".to_string();
        } else if [Some(&"ds_store"), Some(&"localized")].contains(&format) {
            icon = "\u{f179}  ".to_string();
        } else if [Some(&"mp3"), Some(&"ogg")].contains(&format) {
            icon = "\u{f001}  ".to_string();
        } else if [Some(&"doc"), Some(&"docx"), Some(&"gdoc")].contains(&format) {
            icon = "\u{f1c2}  ".to_string();
        } else if [Some(&"tsx"), Some(&"jsx")].contains(&format) {
            icon = "\u{e7ba}  ".to_string();
        } else if [Some(&"properties"), Some(&"json")].contains(&format) {
            icon = "\u{e60b}  ".to_string();
        } else if [Some(&"jar"), Some(&"java")].contains(&format) {
            icon = "\u{e204}  ".to_string();
        } else if [Some(&"lhs"), Some(&"hs")].contains(&format) {
            icon = "\u{e777}  ".to_string();
        } else if [Some(&"mobi"), Some(&"ebook"), Some(&"epub")].contains(&format) {
            icon = "\u{e28b}  ".to_string();
        } else if [Some(&"scss"), Some(&"css")].contains(&format) {
            icon = "\u{e749}  ".to_string();
        } else if [Some(&"editorconfig"), Some(&"conf")].contains(&format) {
            icon = "\u{e615}  ".to_string();
        } else if [Some(&"vim")].contains(&format) {
            icon = "\u{e62b}  ".to_string();
        } else if [Some(&"twig")].contains(&format) {
            icon = "\u{e61c}  ".to_string();
        } else if [Some(&"ts")].contains(&format) {
            icon = "\u{e628}  ".to_string();
        } else if [Some(&"tex")].contains(&format) {
            icon = "\u{e600}  ".to_string();
        } else if [Some(&"sqlite3")].contains(&format) {
            icon = "\u{e7c4}  ".to_string();
        } else if [Some(&"scala")].contains(&format) {
            icon = "\u{e737}  ".to_string();
        } else if [Some(&"sass")].contains(&format) {
            icon = "\u{e603}  ".to_string();
        } else if [Some(&"rss")].contains(&format) {
            icon = "\u{f09e}  ".to_string();
        } else if [Some(&"rdb")].contains(&format) {
            icon = "\u{e76d}  ".to_string();
        } else if [Some(&"psd")].contains(&format) {
            icon = "\u{e7b8}  ".to_string();
        } else if [Some(&"pl")].contains(&format) {
            icon = "\u{e769}  ".to_string();
        } else if [Some(&"php")].contains(&format) {
            icon = "\u{e73d}  ".to_string();
        } else if [Some(&"pdf")].contains(&format) {
            icon = "\u{f1c1}  ".to_string();
        } else if [Some(&"npmignore")].contains(&format) {
            icon = "\u{e71e}  ".to_string();
        } else if [Some(&"mustache")].contains(&format) {
            icon = "\u{e60f}  ".to_string();
        } else if [Some(&"lua")].contains(&format) {
            icon = "\u{e620}  ".to_string();
        } else if [Some(&"log")].contains(&format) {
            icon = "\u{f18d}  ".to_string();
        } else if [Some(&"less")].contains(&format) {
            icon = "\u{e758}  ".to_string();
        } else if [Some(&"js")].contains(&format) {
            icon = "\u{e74e}  ".to_string();
        } else if [Some(&"iml")].contains(&format) {
            icon = "\u{e7b5}  ".to_string();
        } else if [Some(&"html")].contains(&format) {
            icon = "\u{f13b}  ".to_string();
        } else if [Some(&"go")].contains(&format) {
            icon = "\u{e626}  ".to_string();
        } else if [Some(&"gform")].contains(&format) {
            icon = "\u{f298}  ".to_string();
        } else if [Some(&"erl")].contains(&format) {
            icon = "\u{e7b1}  ".to_string();
        } else if [Some(&"ai")].contains(&format) {
            icon = "\u{e7b4}  ".to_string();
        } else if [Some(&"avro")].contains(&format) {
            icon = "\u{e60b}  ".to_string();
        } else if [Some(&"c")].contains(&format) {
            icon = "\u{e61e}  ".to_string();
        } else if [Some(&"clj")].contains(&format) {
            icon = "\u{e768}  ".to_string();
        } else if [Some(&"coffee")].contains(&format) {
            icon = "\u{f0f4}  ".to_string();
        } else if [Some(&"cpp")].contains(&format) {
            icon = "\u{e61d}  ".to_string();
        } else if [Some(&"d")].contains(&format) {
            icon = "\u{e7af}  ".to_string();
        } else if [Some(&"dart")].contains(&format) {
            icon = "\u{e798}  ".to_string();
        } else if [Some(&"db")].contains(&format) {
            icon = "\u{f1c0}  ".to_string();
        } else if [Some(&"diff")].contains(&format) {
            icon = "\u{f440}  ".to_string();
        } else if [Some(&"env"), Some(&"config")].contains(&format) {
            icon = "\u{f462}  ".to_string();
        } else {
            icon = "\u{f15b}  ".to_string();
        }

        escape(icon + &self.file.name.clone(), &mut bits, file_style, self.colours.control_char());
        bits
    }


    /// Figures out which colour to paint the filename part of the output,
    /// depending on which “type” of file it appears to be -- either from the
    /// class on the filesystem or from its name. (Or the broken link colour,
    /// if there’s nowhere else for that fact to be shown.)
    pub fn style(&self) -> Style {
        if let LinkStyle::JustFilenames = self.link_style {
            if let Some(ref target) = self.target {
                if target.is_broken() {
                    return self.colours.broken_symlink();
                }
            }
        }

        self.kind_style()
            .or_else(|| self.exts.colour_file(self.file))
            .unwrap_or_else(|| self.colours.normal())
    }

    fn kind_style(&self) -> Option<Style> {
        Some(match self.file {
            f if f.is_directory()        => self.colours.directory(),
            f if f.is_executable_file()  => self.colours.executable_file(),
            f if f.is_link()             => self.colours.symlink(),
            f if f.is_pipe()             => self.colours.pipe(),
            f if f.is_block_device()     => self.colours.block_device(),
            f if f.is_char_device()      => self.colours.char_device(),
            f if f.is_socket()           => self.colours.socket(),
            f if !f.is_file()            => self.colours.special(),
            _                            => return None,
        })
    }
}


/// The set of colours that are needed to paint a file name.
pub trait Colours: FiletypeColours {

    /// The style to paint the path of a symlink’s target, up to but not
    /// including the file’s name.
    fn symlink_path(&self) -> Style;

    /// The style to paint the arrow between a link and its target.
    fn normal_arrow(&self) -> Style;

	/// The style to paint the filenames of broken links in views that don’t
	/// show link targets, and the style to paint the *arrow* between the link
	/// and its target in views that *do* show link targets.
    fn broken_symlink(&self) -> Style;

    /// The style to paint the entire filename of a broken link.
    fn broken_filename(&self) -> Style;

    /// The style to paint a non-displayable control character in a filename.
    fn control_char(&self) -> Style;

    /// The style to paint a non-displayable control character in a filename,
    /// when the filename is being displayed as a broken link target.
    fn broken_control_char(&self) -> Style;

    /// The style to paint a file that has its executable bit set.
    fn executable_file(&self) -> Style;
}


// needs Debug because FileStyle derives it
use std::fmt::Debug;
use std::marker::Sync;
pub trait FileColours: Debug+Sync {
    fn colour_file(&self, file: &File) -> Option<Style>;
}


#[derive(PartialEq, Debug)]
pub struct NoFileColours;
impl FileColours for NoFileColours {
    fn colour_file(&self, _file: &File) -> Option<Style> { None }
}

// When getting the colour of a file from a *pair* of colourisers, try the
// first one then try the second one. This lets the user provide their own
// file type associations, while falling back to the default set if not set
// explicitly.
impl<A, B> FileColours for (A, B)
where A: FileColours, B: FileColours {
    fn colour_file(&self, file: &File) -> Option<Style> {
        self.0.colour_file(file).or_else(|| self.1.colour_file(file))
    }
}
