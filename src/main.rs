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

    pub fn get_byte(&self, u32) -> u8 {
        self.buf[u32]
    } 
} 

/// We represent pieces by their index in the vector that we use to allocate 
/// them.  That is fine because we never free a piece anyway (unlimited undo
/// for the win).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Piece(u32);

/// The actual data stored in a piece.  
/// We have one sentinel piece which is always stored at index 0
/// in the vector.  It's span is also empty
#[derive(Debug)]
struct PieceData {
    /// Some bytes in the text's buffer
    span: Span,
    prev: Piece,
    next: Piece,
} 

/// Text is just a sequence of bytes (implemented with the PieceTable method,
/// ala Oberon).  We on purpose do not require UTF-8 here.  A programmers
/// editor is most useful when it can deal with any sequence of bytes.
#[derive(Debug)]
pub struct Text {
    buffer: AppendOnlyBuffer,
    pieces: Vec<PieceData>,
    len: usize,
} 

struct Pieces<'a> {
    text: &'a Text,
    next: Piece,
    /// start position of piece in text
    off: u32, 
} 

impl<'a> Iterator for Pieces<'a> {
    type Item = (u32, Piece);

    fn next(&mut self) -> Option<(u32, Piece)> {
        if self.next == SENTINEL {
            None
        } else {
            let piece = self.next;
            let Piece(p) = piece;
            let pd = &self.text.pieces[p as usize];
            let off = self.off;
            let span = &pd.span;
            let next = *&pd.next;
            self.off = self.off + span.len();
            self.next = next;
            Some ((off, piece))
        } 
    } 
} 

struct Bytes<'a> {
    pieces: Pieces<'a>,
    pd: Option<&'a PieceData>;
    // where we are in the current piece
    off: u32
} 

impl<'a> Iterator for Bytes<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        match self.pd {
            None => None,
            Some(pd) => {
                let span = pd.span in
                if self.off >= span.len() {
                    self.off = 0;
                    self.pd = self.pieces.next();
                    self.next();
                } else {
                    let byte = self.pieces.text.buffer.get_byte(span.off1 + self.off);
                    self.off += 1;
                    Some(byte)
                } 
            } 
        } 
    } 
} 

// The sentinel is always stored at position 0 in the pieces vector
const SENTINEL: Piece = Piece(0);

impl Text {
    pub fn new() -> Text {
        Text {
            buffer: AppendOnlyBuffer::new(),
            pieces: vec![PieceData { 
                span: Span::empty(),
                prev: SENTINEL,
                next: SENTINEL,
            }],
            len: 0,
        } 
    } 

    fn invariant(&self) {
        let mut l = 0;
        let mut p = self.get_piece(SENTINEL).next;
        while p != SENTINEL {
            let len = self.get_piece(p).span.len();
            assert!(len > 0);
            l += len;
            p = self.get_piece(p).next;
        } 
        assert_eq!(l as usize, self.len());

        let mut l = 0;
        let mut p = self.get_piece(SENTINEL).prev;
        while p != SENTINEL {
            let len = self.get_piece(p).span.len();
            assert!(len > 0);
            l += len;
            p = self.get_piece(p).prev;
        } 
        assert_eq!(l as usize, self.len());
    } 

    /// Iterator over all pieces (but never the sentinel)
    fn pieces(&self) -> Pieces {
        let next = self.get_piece(SENTINEL).next;
        Pieces {
            text: self,
            next: next,
            off: 0,
        } 
    } 

    /// Length of Text in bytes
    pub fn len(&self) -> usize {
        self.len
    } 

    fn get_piece(&self, Piece(p): Piece) -> &PieceData {
        &self.pieces[p as usize]
    } 

    fn link(&mut self, piece1: Piece, piece2: Piece) {
        let Piece(p1) = piece1;
        let Piece(p2) = piece2;
        self.pieces[p1 as usize].next = piece2;
        self.pieces[p2 as usize].prev = piece1;
    } 

    /// Find the piece containing offset.  Return piece
    /// and start position of piece in text.
    /// Will return the sentinel iff off == self.len()
    /// Returns the right piece if off between two
    /// pieces
    fn find_piece(&self, off:u32) -> (u32, Piece) {
        if off == self.len() as u32 {
            (off, SENTINEL)
        } else { 
            let mut start = 0;
            let mut piece = SENTINEL;
            for (s, p) in self.pieces() {
                if s > off {
                    // previous piece was the one we wanted
                    return (start, piece);
                } 
                start = s;
                piece = p;
            }
            return (start, piece);
        } 
    } 

    fn add_piece(&mut self, span: Span) -> Piece {
        self.pieces.push(PieceData { 
            span: span, 
            prev: SENTINEL, 
            next: SENTINEL,
        } );
        Piece((self.pieces.len() - 1) as u32)
    } 

    /// Delete bytes between off1 (inclusive) and off2 (exclusive)
    pub fn delete(&mut self, off1: u32, off2: u32) {
        if off2 <= off1 {
            return;
        } 
        let (lstart, lpiece) = self.find_piece(off1);
        let lspan = self.get_piece(lpiece).span; 
        let (rstart, rpiece) = self.find_piece(off2);
        let rspan = self.get_piece(rpiece).span; 
        let left = {
            if let Some((left_span, _right_span)) = lspan.split(off1 - lstart) {
                let l = self.get_piece(lpiece).prev;
                let remainder = self.add_piece(left_span);
                self.link(l, remainder);
                remainder
            } else {
                // We are deleting all of piece
                assert_eq!(lstart, off1);
                self.get_piece(lpiece).prev
            } 
        };
        let right = {
            if let Some((_left_span, right_span)) = rspan.split(off2 - rstart) {
                let r = self.get_piece(rpiece).next;
                let remainder = self.add_piece(right_span);
                self.link(remainder, r);
                remainder
            } else {
                // We are at the beginning of piece and therefore
                // won't delete anything of it
                assert_eq!(rstart, off2);
                rpiece
            } 
        };
        self.len -= (off2 - off1) as usize;
        self.link(left, right);
        self.invariant()
    } 

    /// Insert bytes at offset.
    pub fn insert(&mut self, off:u32, bytes: &[u8]) {
        if bytes.len() == 0 {
            return;
        } 
        let (start, piece) = self.find_piece(off);
        let (span, prev, next) = {
            let d = self.get_piece(piece);
            (d.span, d.prev, d.next)
        };
        if let Some((left_span, right_span)) = span.split(off - start) {
            let left = self.add_piece(left_span);
            let span = self.buffer.append(bytes);
            let middle = self.add_piece(span);
            let right = self.add_piece(right_span);
            self.link(prev, left);
            self.link(left, middle);
            self.link(middle, right);
            self.link(right, next);
        } else {
            // insert at beginning aka in front of the piece
            assert_eq!(start, off);
            let span = self.buffer.append(bytes);
            let p = self.add_piece(span);
            self.link(p, piece);
            self.link(prev, p);
        } 
        self.len = self.len + bytes.len();
        self.invariant();
    } 

    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for (_, p) in self.pieces() {
            v.push_all(self.buffer.get(self.get_piece(p).span))
        } 
        v
    } 

    pub fn to_utf8_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.to_vec())
    } 
} 

#[test]
fn test_pieces() {
    let t = Text::new();
    assert_eq!(t.pieces().collect::<Vec<_>>(), vec![]);
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
        fn insert_beginning() {
            let mut t = Text::new();
            assert_eq!(t.len(), 0);
            t.insert(0, "World".as_bytes());
            assert_eq!(t.len(), 5);
            assert_eq!(t.to_utf8_string().unwrap(), "World");
            t.insert(0, "Hello ".as_bytes());
            assert_eq!(t.len(), 11);
            assert_eq!(t.to_utf8_string().unwrap(), "Hello World");
            t.insert(0, "...".as_bytes());
            assert_eq!(t.len(), 14);
            assert_eq!(t.to_utf8_string().unwrap(), "...Hello World");
        } 

        #[test]
        fn append() {
            let mut t = Text::new();
            t.insert(0, "Hello".as_bytes());
            assert_eq!(t.to_utf8_string().unwrap(), "Hello");
            t.insert(5, " Bene".as_bytes());
            assert_eq!(t.to_utf8_string().unwrap(), "Hello Bene");
        } 

        #[test]
        fn insert_middle() {
            let mut t = Text::new();
            t.insert(0, "1234".as_bytes());
            t.insert(2, "x".as_bytes());
            assert_eq!(t.to_utf8_string().unwrap(), "12x34");
            t.insert(3, "yz".as_bytes());
            assert_eq!(t.to_utf8_string().unwrap(), "12xyz34");
        }

        #[test]
        fn delete_all1() {
            let mut t = Text::new();
            t.insert(0, "123456".as_bytes());
            t.delete(0, 6);
            assert_eq!(t.len(), 0);
        } 

        #[test]
        fn delete_all2() {
            let mut t = Text::new();
            t.insert(0, "456".as_bytes());
            t.insert(0, "123".as_bytes());
            t.delete(0, 6);
            assert_eq!(t.len(), 0);
        } 

        #[test]
        fn delete_part1() {
            let mut t = Text::new();
            t.insert(0, "123456".as_bytes());
            t.delete(1, 5);
            assert_eq!(t.len(), 2);
            assert_eq!(t.to_utf8_string().unwrap(), "16");
        } 

        #[test]
        fn delete_part2() {
            let mut t = Text::new();
            t.insert(0, "456".as_bytes());
            t.insert(0, "123".as_bytes());
            t.delete(1, 5);
            assert_eq!(t.len(), 2);
            assert_eq!(t.to_utf8_string().unwrap(), "16");
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
