#[macro_use]
extern crate log;
extern crate env_logger;

/// A append only buffer
#[derive(Debug)]
struct AppendOnlyBuffer {
    buf: String,
} 

#[derive(Debug,Copy,Clone)]
struct Span {
    off1: u32,
    off2: u32,
} 

impl Span {
    pub fn new(off1: u32, off2: u32) -> Span {
        assert!(off2 > off1);
        Span { off1: off1, off2: off2 }
    } 

    pub fn len(&self) -> u32 {
        self.off2 - self.off1 
    } 
} 

impl AppendOnlyBuffer {
    /// Constructs a new, empty AppendOnlyBuffer.
    pub fn new() -> AppendOnlyBuffer {
        AppendOnlyBuffer {
          buf : String::with_capacity(4096)
        } 
    }

    /// Append a string.
    pub fn append(&mut self, s: &str) -> Span {
      let off1 = self.buf.len() as u32;
      self.buf.push_str(s);
      Span::new(off1, self.buf.len() as u32)
    } 

    pub fn get(&self, s: Span) -> &str {
        // We know by construction that all Span's constructed by append
        // are valid UTF-8 strings
        unsafe {
            self.buf.slice_unchecked(s.off1 as usize, s.off2 as usize)
        } 
    } 
} 

#[derive(Debug, Copy, Clone)]
struct Piece(u32);

#[derive(Debug)]
struct PieceData {
    span: Span,
    prev: Option<Piece>,
    next: Option<Piece>,
} 

impl PieceData {
    pub fn new(s: Span) -> PieceData {
        PieceData {
            span: s,
            prev: None,
            next: None
        } 
    } 
} 

/// A Text buffer (implemented with the PieceTable method)
/// There is always at least one piece (which might be empty)
#[derive(Debug)]
pub struct Text {
    buffer: AppendOnlyBuffer,
    pieces: Vec<PieceData>,
    first: Piece, 
} 

/// A iterator over each piece
struct Pieces<'a> {
    text: &'a Text,
    curr: Option<Piece>,
    /// start position of piece in text
    off: u32, 
} 

impl<'a> Iterator for Pieces<'a> {
    type Item = (u32, Piece);

    fn next(&mut self) -> Option<(u32, Piece)> {
        match self.curr {
            None => None,
            Some(piece) => {
                let Piece(p) = piece;
                let pd = &self.text.pieces[p as usize];
                let off = self.off;
                let span = &pd.span;
                let next = *&pd.next;
                self.off = self.off + span.len();
                self.curr = next;
                Some ((off, piece))
            } 
        } 
    } 
} 

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

    /// Find the piece containing offset.  Return piece
    /// and start position of piece in text.
    fn piece_containing(&self, off:u32) -> Option<(u32, Piece)> {
        let mut start = 0;
        let mut piece = self.first;
        for (s, p) in self.iter_pieces() {
            if s <= off && off < s + self.get_piece(p).span.len() {
                return Some((s, p));
            } 
        }
        None
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
