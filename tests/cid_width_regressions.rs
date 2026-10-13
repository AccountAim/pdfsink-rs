use lopdf::{content::{Content, Operation}, dictionary, Document, Object, Stream};

fn fixture(widths: Vec<Object>) -> tempfile::NamedTempFile {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let descriptor = doc.add_object(dictionary! {"Type"=>"FontDescriptor", "FontName"=>"TestFont",
        "Flags"=>4, "FontBBox"=>vec![0.into(),0.into(),1000.into(),1000.into()],
        "ItalicAngle"=>0, "Ascent"=>800, "Descent"=>-200, "CapHeight"=>700, "StemV"=>80});
    let descendant = doc.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"CIDFontType2",
        "BaseFont"=>"TestFont", "FontDescriptor"=>descriptor, "DW"=>1000, "W"=>widths,
        "CIDSystemInfo"=>dictionary! {"Registry"=>Object::string_literal("Adobe"),
            "Ordering"=>Object::string_literal("Identity"), "Supplement"=>0}});
    let cmap = doc.add_object(Stream::new(dictionary! {}, b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /Test def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange 1 beginbfrange <0020> <007E> <0020> endbfrange endcmap CMapName currentdict /CMap defineresource pop end end".to_vec()));
    let font = doc.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type0", "BaseFont"=>"TestFont",
        "Encoding"=>"Identity-H", "DescendantFonts"=>vec![descendant.into()], "ToUnicode"=>cmap});
    let encoded = |s: &str| Object::String(s.encode_utf16().flat_map(u16::to_be_bytes).collect(), lopdf::StringFormat::Hexadecimal);
    let mut operations = vec![Operation::new("BT",vec![]), Operation::new("Tf",vec![Object::Name(b"F1".to_vec()),12.into()])];
    for (x, text) in [(10.0, "www"), (31.6, ".example.org"), (250.0, "~") ] {
        operations.push(Operation::new("Tm",vec![1.into(),0.into(),0.into(),1.into(),Object::Real(x),80.into()]));
        operations.push(Operation::new("Tj",vec![encoded(text)]));
    }
    operations.push(Operation::new("ET",vec![]));
    let content = doc.add_object(Stream::new(dictionary! {},Content{operations}.encode().unwrap()));
    let page = doc.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),300.into(),100.into()],
        "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}, "Contents"=>content});
    doc.objects.insert(pages,Object::Dictionary(dictionary! {"Type"=>"Pages", "Kids"=>vec![page.into()], "Count"=>1}));
    let root = doc.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});doc.trailer.set("Root",root);
    let file = tempfile::NamedTempFile::new().unwrap();doc.save(file.path()).unwrap();file
}

#[test]
fn cid_range_widths_equal_array_widths_and_include_last_character() {
    let range = fixture(vec![32.into(),126.into(),600.into()]);
    let array = fixture(vec![32.into(),Object::Array(vec![600.into();95])]);
    let range_doc = pdfsink_rs::PdfDocument::open(range.path()).unwrap();
    let array_doc = pdfsink_rs::PdfDocument::open(array.path()).unwrap();
    let a = range_doc.page(1).unwrap();let b = array_doc.page(1).unwrap();
    assert_eq!(a.chars.len(),b.chars.len());
    for (a,b) in a.chars.iter().zip(&b.chars) {
        assert_eq!(a.text,b.text);
        assert!((a.x0-b.x0).abs()<0.001,"{} origins: {} vs {}",a.text,a.x0,b.x0);
        assert!((a.x1-b.x1).abs()<0.001,"{} ends: {} vs {}",a.text,a.x1,b.x1);
    }
    assert_eq!(a.extract_text(),b.extract_text());
    assert!(a.extract_text().contains("www.example.org"));
    let last = a.chars.iter().find(|c|c.text=="~").unwrap();
    assert!((last.x1-last.x0-7.2).abs()<0.001);
}

#[test]
fn singleton_range_retains_its_declared_width() {
    let file = fixture(vec![126.into(),126.into(),600.into()]);
    let doc = pdfsink_rs::PdfDocument::open(file.path()).unwrap();
    let last = doc.page(1).unwrap().chars.iter().find(|c|c.text=="~").unwrap();
    assert!((last.x1-last.x0-7.2).abs()<0.001);
}
