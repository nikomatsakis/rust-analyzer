use std::{fmt, ops::Deref, ptr, sync::Arc};

use {
    yellow::{GreenNode, RedNode},
    SyntaxKind::{self, *},
    TextRange, TextUnit,
};

pub trait TreeRoot: Deref<Target = SyntaxRoot> + Clone {}
impl TreeRoot for Arc<SyntaxRoot> {}
impl<'a> TreeRoot for &'a SyntaxRoot {}

#[derive(Clone, Copy)]
pub struct SyntaxNode<R: TreeRoot = Arc<SyntaxRoot>> {
    pub(crate) root: R,
    // Guaranteed to not dangle, because `root` holds a
    // strong reference to red's ancestor
    red: ptr::NonNull<RedNode>,
}

pub type SyntaxNodeRef<'a> = SyntaxNode<&'a SyntaxRoot>;

#[derive(Debug)]
pub struct SyntaxRoot {
    red: RedNode,
    pub(crate) errors: Vec<SyntaxError>,
}

impl SyntaxRoot {
    pub(crate) fn new(green: GreenNode, errors: Vec<SyntaxError>) -> SyntaxRoot {
        SyntaxRoot {
            red: RedNode::new_root(green),
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct SyntaxError {
    pub(crate) message: String,
    pub(crate) offset: TextUnit,
}

impl SyntaxNode<Arc<SyntaxRoot>> {
    pub(crate) fn new_owned(root: SyntaxRoot) -> Self {
        let root = Arc::new(root);
        let red_weak = ptr::NonNull::from(&root.red);
        SyntaxNode {
            root,
            red: red_weak,
        }
    }
}

impl<R: TreeRoot> SyntaxNode<R> {
    pub fn borrow<'a>(&'a self) -> SyntaxNode<&'a SyntaxRoot> {
        SyntaxNode {
            root: &*self.root,
            red: ptr::NonNull::clone(&self.red),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.red().green().kind()
    }

    pub fn range(&self) -> TextRange {
        let red = self.red();
        TextRange::offset_len(red.start_offset(), red.green().text_len())
    }

    pub fn text(&self) -> String {
        self.red().green().text()
    }

    pub fn children<'a>(&'a self) -> impl Iterator<Item=SyntaxNode<R>> + 'a {
        let red = self.red();
        let n_children = red.n_children();
        (0..n_children).map(move |i| {
            SyntaxNode {
                root: self.root.clone(),
                red: red.nth_child(i),
            }
        })
    }

    pub fn parent(&self) -> Option<SyntaxNode<R>> {
        let parent = self.red().parent()?;
        Some(SyntaxNode {
            root: self.root.clone(),
            red: parent,
        })
    }

    fn red(&self) -> &RedNode {
        unsafe { self.red.as_ref() }
    }
}

impl<R: TreeRoot> fmt::Debug for SyntaxNode<R> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}@{:?}", self.kind(), self.range())?;
        if has_short_text(self.kind()) {
            write!(fmt, " \"{}\"", self.text())?;
        }
        Ok(())
    }
}

fn has_short_text(kind: SyntaxKind) -> bool {
    match kind {
        IDENT | LIFETIME => true,
        _ => false,
    }
}