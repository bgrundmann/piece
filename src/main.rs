#![feature(collections)]
#[macro_use]
extern crate log;
extern crate env_logger;

/// A append only buffer
#[derive(Debug)]
pub struct AppendOnlyBuffer {
    buf: Vec<u8>,
} 

#[derive(Debug,Copy,Clone,PartialEq)]
pub struct Span {
    off1: u32,
    off2: u32,
} 

impl Span {
    pub fn new(off1: u32, off2: u32) -> Span {
        assert!(off2 >= off1);
        Span { off1: off1, off2: off2 }
    } 

    /// The empty span 
    pub fn empty() -> Span {
        Span::new(0,0)
    } 

    pub fn len(&self) -> u32 {
        self.off2 - self.off1 
    } 

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Split self such that the left piece has n characters.
    pub fn split(&self, n: u32) -> Option<(Span, Span)> {
        if n == 0 || n == self.len() {
            None
        } else {
            Some((Span::new(self.off1, self.off1+n), Span::new(self.off1+n, self.off2)))
        } 
    } 
} 

impl AppendOnlyBuffer {
    /// Constructs a new, empty AppendOnlyBuffer.
    pub fn new() -> AppendOnlyBuffer {
        AppendOnlyBuffer {
          buf: Vec::with_capacity(4096)
        } 
    }

    /// Append a slice of bytes.
    pub fn append(&mut self, bytes: &[u8]) -> Span {
      let off1 = self.buf.len() as u32;
      self.buf.push_all(bytes);
      Span::new(off1, self.buf.len() as u32)
    } 

    pub fn get(&self, s: Span) -> &[u8] {
        &self.buf[s.off1 as usize .. s.off2 as usize]
    } 
} 

/// We represent pieces by their index in the vector that we use to allocate 
/// them.  That is fine because we never free a piece anyway (unlimited undo
/// for the win).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Piece(u32);

/// The actual data stored in a piece.  
#[derive(Debug)]
struct PieceData {
    /// Some bytes in the text's buffer
    span: Span,
    prev: Piece,
    next: Piece,
    /// The last piece is marked trailer.  It is also
    /// always empty (span.is_empty())
    trailer: bool,
} 

/// Text is just a sequence of bytes (implemented with the PieceTable method,
/// ala Oberon).  We on purpose do not require UTF-8 here.  A programmers
/// editor is most useful when it can deal with any sequence of bytes.
#[derive(Debug)]
pub struct Text {
    buffer: AppendOnlyBuffer,
    pieces: Vec<PieceData>,
    first: Piece, 
} 

struct Pieces<'a> {
    text: &'a Text,
    curr: Piece,
    /// start position of piece in text
    off: u32, 
    /// Has trailer been emited?
    done: bool,
} 

impl<'a> Iterator for Pieces<'a> {
    type Item = (u32, Piece);

    fn next(&mut self) -> Option<(u32, Piece)> {
        if self.done {
            None
        } else {
            let piece = self.curr;
            let Piece(p) = piece;
            let pd = &self.text.pieces[p as usize];
            let off = self.off;
            let span = &pd.span;
            let next = *&pd.next;
            self.off = self.off + span.len();
            self.curr = next;
            self.done = pd.trailer;
            Some ((off, piece))
        } 
    } 
} 

impl Text {
    pub fn new() -> Text {
        let trailer = Piece(0);
        Text {
            buffer: AppendOnlyBuffer::new(),
            pieces: vec![PieceData { 
                span: Span::empty(),
                prev: trailer,
                next: trailer,
                trailer: true,
            }],
            first: trailer,
        } 
    } 

    fn pieces(&self) -> Pieces {
        Pieces {
            text: self,
            curr: self.first,
            off: 0,
            done: false,
        } 
    } 

    fn get_piece(&self, Piece(p): Piece) -> &PieceData {
        &self.pieces[p as usize]
    } 

    /// Find the piece containing offset.  Return piece
    /// and start position of piece in text.
    fn find_piece(&self, off:u32) -> (u32, Piece) {
        let mut start = 0;
        let mut piece = self.first;
        for (s, p) in self.pieces() {
            if s > off || self.get_piece(p).trailer {
                // previous piece was the one we wanted
                return (start, piece);
            } 
            start = s;
            piece = p;
        }
        unreachable!();
    } 

    /// Insert bytes at offset.
    pub fn insert(&mut self, off:u32, bytes: &[u8]) {
        let (start, piece) = self.find_piece(off);
        let span = self.get_piece(piece).span;
        if let Some((left_span, right_span)) = span.split(off - start) {
            unreachable!();
        } else {
            // insert at beginning
            assert_eq!(start, off);
            let span = self.buffer.append(bytes);
        } 
    } 
} 

#[test]
fn test_pieces() {
    let t = Text::new();
    assert_eq!(t.pieces().collect::<Vec<_>>(), vec![(0, Piece(0))]);
} 

#[cfg(test)]
mod tests {
    mod span {
        use super::super::*;

        #[test]
        fn basics() {
            let s = Span::new(1, 1);
            assert_eq!(s.len(), 0);
            assert!(s.is_empty());
            let s2 = Span::new(3, 7);
            assert!(s2.len() == 4);
        } 

        #[test]
        fn split() {
            let s = Span::new(3, 7);
            assert_eq!(s.split(0), None);
            assert_eq!(s.split(4), None);
            assert_eq!(s.split(3), Some((Span { off1: 3, off2: 6 }, Span { off1: 6, off2: 7 })));
        } 
    } 

    mod append_only_buffer {
        use super::super::*;

        #[test] 
        fn basics() {
            let mut b = AppendOnlyBuffer::new();
            let bytes = "Hello World".as_bytes();
            let sp = b.append(bytes);
            assert_eq!(b.get(sp), bytes);
            let bytes2 = "Just testing".as_bytes();
            let sp2 = b.append(bytes2);
            assert_eq!(b.get(sp), bytes);
            assert_eq!(b.get(sp2), bytes2);
        } 
    } 

    mod text {
        use super::super::*;

        #[test]
        fn basics() {
            let mut t = Text::new();
            t.insert(0, "Hello".as_bytes());
        } 
    } 
}
/*



impl Text {
    pub fn from_str(s: &str) -> Text {
        let mut buffer = AppendOnlyBuffer::new();
        let span       = buffer.append(s);
        let piece_data = PieceData::new(span);
        Text {
          buffer: buffer,
          pieces: vec![piece_data],
          first: Piece(0)
        } 
    } 

    fn get_mut_piece(&mut self, Piece(p1): Piece) -> &mut PieceData {
        &mut self.pieces[p1 as usize]
    } 

    fn get_piece(&self, Piece(p1): Piece) -> &PieceData {
        &self.pieces[p1 as usize]
    } 

    fn iter_pieces(&self) -> Pieces {
        Pieces {
            text: self,
            curr: Some(self.first),
            off:  0,
        } 
    } 

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        let spans = self.iter_pieces()
            .map(|(_, piece)| self.get_piece(piece).span);
        for span in spans {
            result.push_str(self.buffer.get(span));
        } 
        result
    } 

    fn last_piece(&self) -> (u32, Piece) {
        let mut off = 0;
        let mut piece = self.first;
        for (o, p) in self.iter_pieces() {
            off = o;
            piece = p;
        } 
        (off, piece)
    } 


    fn link(&mut self, p1: Piece, p2: Piece) {
        self.get_mut_piece(p1).next = Some(p2);
        self.get_mut_piece(p2).prev = Some(p1);
    } 

    pub fn append(&mut self, s: &str) {
        if s.len() > 0 {
            let (_, old_last_piece) = self.last_piece();
            let span       = self.buffer.append(s);
            let piece_data = PieceData::new(span);
            self.pieces.push(piece_data);
            let new_last_piece = Piece( (self.pieces.len() - 1) as u32);
            self.link(old_last_piece, new_last_piece)
        } 
    } 

    pub fn delete(&mut self, span:Span) {
        //  0123  456  789
        // [XXYY][YYY][YXX]
        // 
        // delete [2-8)
        //
        match (self.piece_containing(span.off1), self.piece_containing(span.off2)) {
            None, None    => panic!("invalid span to delete"),
            None, Some(_) => panic!("invalid span to delete"),
            Some(_), None => panic!("invalid span to delete"),
            Some ((start1, piece1)), Some ((start2, piece2)) => {
                if (piece1 = piece2) {
                    // special case deletion in one piece

                } 
            } 
        } 
    } 
} 

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_str_test() {
        let text = Text::from_str("Hello");
        assert_eq!(text.to_string(), "Hello");
    } 

    #[test]
    fn append_test() {
        let mut text = Text::from_str("Hello");
        text.append(" ");
        text.append("World");
        assert_eq!(text.to_string(), "Hello World");
    } 

    #[test]
    fn iter_offset_test() {
        let mut text = Text::from_str("Hello");
        text.append(" ");
        text.append("World");
        let expected = vec![0, 5, 6];
        let actual: Vec<_> = text.iter_pieces().map(|(o,_)| o).collect();
        assert_eq!(actual, expected);
    } 
} 

fn main() {
    env_logger::init().unwrap();
    info!("starting up");

    let mut text = Text::from_str("Hello");
    text.append(" ");
    text.append("World!");
    println!("{:?}", text);
    for (off, piece) in text.iter_pieces() {
        println!("{}: {:?}", off, piece);
    } 
    println!("{}", text.to_string());
}
*/
