#[cfg(any(test, target_os = "macos"))]
use crate::validate_file;
use crate::{Frame, MediaError};
use std::path::Path;

#[cfg(any(test, target_os = "macos"))]
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(any(test, target_os = "macos"))]
const MAX_PAGES: u32 = 2_000;
#[cfg(any(test, target_os = "macos"))]
const MAX_DIMENSION: u32 = 8_192;

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::{ffi::c_void, os::unix::ffi::OsStrExt, ptr};

    type CFIndex = isize;
    type CFBoolean = u8;
    type CFAllocatorRef = *const c_void;
    type CFURLRef = *const c_void;
    type CGPDFDocumentRef = *mut c_void;
    type CGPDFPageRef = *mut c_void;
    type CGColorSpaceRef = *mut c_void;
    type CGContextRef = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(dead_code)]
    struct CGAffineTransform {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        tx: f64,
        ty: f64,
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: CFAllocatorRef,
            buffer: *const u8,
            buf_len: CFIndex,
            is_directory: CFBoolean,
        ) -> CFURLRef;
        fn CFRelease(cf: *const c_void);
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPDFDocumentCreateWithURL(url: CFURLRef) -> CGPDFDocumentRef;
        fn CGPDFDocumentGetNumberOfPages(document: CGPDFDocumentRef) -> usize;
        fn CGPDFDocumentGetPage(document: CGPDFDocumentRef, page: usize) -> CGPDFPageRef;
        fn CGPDFDocumentRelease(document: CGPDFDocumentRef);
        fn CGPDFPageGetBoxRect(page: CGPDFPageRef, box_type: i32) -> CGRect;
        fn CGPDFPageGetRotationAngle(page: CGPDFPageRef) -> i32;
        fn CGPDFPageGetDrawingTransform(
            page: CGPDFPageRef,
            box_type: i32,
            rect: CGRect,
            rotate: i32,
            preserve_aspect_ratio: bool,
        ) -> CGAffineTransform;
        fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
        fn CGColorSpaceRelease(space: CGColorSpaceRef);
        fn CGBitmapContextCreate(
            data: *mut c_void,
            width: usize,
            height: usize,
            bits_per_component: usize,
            bytes_per_row: usize,
            space: CGColorSpaceRef,
            bitmap_info: u32,
        ) -> CGContextRef;
        fn CGContextRelease(context: CGContextRef);
        fn CGContextSetRGBFillColor(context: CGContextRef, r: f64, g: f64, b: f64, a: f64);
        fn CGContextFillRect(context: CGContextRef, rect: CGRect);
        fn CGContextConcatCTM(context: CGContextRef, transform: CGAffineTransform);
        fn CGContextScaleCTM(context: CGContextRef, sx: f64, sy: f64);
        fn CGContextDrawPDFPage(context: CGContextRef, page: CGPDFPageRef);
    }

    struct Document(CGPDFDocumentRef);
    impl Drop for Document {
        fn drop(&mut self) {
            unsafe { CGPDFDocumentRelease(self.0) }
        }
    }
    struct Url(CFURLRef);
    impl Drop for Url {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) }
        }
    }

    fn document(path: &Path) -> Result<Document, MediaError> {
        validate_pdf_file(path)?;
        let bytes = path.as_os_str().as_bytes();
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                ptr::null(),
                bytes.as_ptr(),
                bytes.len() as CFIndex,
                0,
            )
        };
        if url.is_null() {
            return Err(MediaError::InvalidPath(format!(
                "could not create PDF URL for {}",
                path.display()
            )));
        }
        let url = Url(url);
        let document = unsafe { CGPDFDocumentCreateWithURL(url.0) };
        if document.is_null() {
            return Err(MediaError::InvalidMedia(format!(
                "{} is not a readable PDF",
                path.display()
            )));
        }
        let document = Document(document);
        let count = unsafe { CGPDFDocumentGetNumberOfPages(document.0) };
        if count == 0 || count > MAX_PAGES as usize {
            return Err(MediaError::InvalidMedia(format!(
                "PDF page count must be between 1 and {MAX_PAGES}"
            )));
        }
        Ok(document)
    }

    pub(super) fn page_count(path: &Path) -> Result<u32, MediaError> {
        let doc = document(path)?;
        Ok(unsafe { CGPDFDocumentGetNumberOfPages(doc.0) as u32 })
    }

    pub(super) fn render(
        path: &Path,
        page_number: u32,
        max_w: u32,
        max_h: u32,
    ) -> Result<Frame, MediaError> {
        validate_dimensions(max_w, max_h)?;
        let doc = document(path)?;
        let count = unsafe { CGPDFDocumentGetNumberOfPages(doc.0) as u32 };
        if page_number == 0 || page_number > count {
            return Err(MediaError::InvalidOptions(format!(
                "PDF page {page_number} is outside 1..={count}"
            )));
        }
        let page = unsafe { CGPDFDocumentGetPage(doc.0, page_number as usize) };
        if page.is_null() {
            return Err(MediaError::InvalidMedia(format!(
                "could not read PDF page {page_number}"
            )));
        }
        // CGPDFPageCropBox is enum value 1.
        let crop = unsafe { CGPDFPageGetBoxRect(page, 1) };
        if !crop.size.width.is_finite()
            || !crop.size.height.is_finite()
            || crop.size.width <= 0.0
            || crop.size.height <= 0.0
        {
            return Err(MediaError::InvalidMedia(
                "PDF page has an invalid crop box".into(),
            ));
        }
        let rotation = unsafe { CGPDFPageGetRotationAngle(page) }.rem_euclid(360);
        let quarter_turn = rotation == 90 || rotation == 270;
        let (page_w, page_h) = if quarter_turn {
            (crop.size.height, crop.size.width)
        } else {
            (crop.size.width, crop.size.height)
        };
        let scale = (max_w as f64 / page_w).min(max_h as f64 / page_h);
        let width = (page_w * scale).round().clamp(1.0, max_w as f64) as u32;
        let height = (page_h * scale).round().clamp(1.0, max_h as f64) as u32;
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| {
                MediaError::InvalidOptions("PDF frame size overflows address space".into())
            })?;
        let mut rgba = vec![255u8; len];
        let color = unsafe { CGColorSpaceCreateDeviceRGB() };
        if color.is_null() {
            return Err(MediaError::Process(
                "CoreGraphics could not create RGB color space".into(),
            ));
        }
        let context = unsafe {
            CGBitmapContextCreate(
                rgba.as_mut_ptr().cast(),
                width as usize,
                height as usize,
                8,
                width as usize * 4,
                color,
                1,
            )
        };
        unsafe { CGColorSpaceRelease(color) };
        if context.is_null() {
            return Err(MediaError::Process(
                "CoreGraphics could not create PDF bitmap context".into(),
            ));
        }
        struct Context(CGContextRef);
        impl Drop for Context {
            fn drop(&mut self) {
                unsafe { CGContextRelease(self.0) }
            }
        }
        let context = Context(context);
        let out = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: width as f64,
                height: height as f64,
            },
        };
        unsafe {
            CGContextSetRGBFillColor(context.0, 1.0, 1.0, 1.0, 1.0);
            CGContextFillRect(context.0, out);
            // A CGBitmapContext already stores row 0 as the visual top, so no flip is needed.
            // CGPDFPageGetDrawingTransform never scales up, so map the page at its own size
            // and scale the context: a 720×405 pt slide must fill a 2560 px output.
            CGContextScaleCTM(context.0, width as f64 / page_w, height as f64 / page_h);
            let rect = CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: page_w,
                    height: page_h,
                },
            };
            let transform = CGPDFPageGetDrawingTransform(page, 1, rect, 0, true);
            CGContextConcatCTM(context.0, transform);
            CGContextDrawPDFPage(context.0, page);
        }
        Ok(Frame {
            width,
            height,
            rgba,
            sequence: 0,
        })
    }
}

#[cfg(any(test, target_os = "macos"))]
fn validate_pdf_file(path: &Path) -> Result<(), MediaError> {
    validate_file(path)?;
    let size = std::fs::metadata(path)?.len();
    if size > MAX_FILE_BYTES {
        return Err(MediaError::InvalidMedia(
            "PDF exceeds the 512 MiB size limit".into(),
        ));
    }
    if size == 0 {
        return Err(MediaError::InvalidMedia("PDF is empty".into()));
    }
    Ok(())
}

#[cfg(any(test, target_os = "macos"))]
fn validate_dimensions(max_w: u32, max_h: u32) -> Result<(), MediaError> {
    if max_w == 0 || max_h == 0 || max_w > MAX_DIMENSION || max_h > MAX_DIMENSION {
        return Err(MediaError::InvalidOptions(
            "PDF dimensions must be within 1×1 and 8192×8192".into(),
        ));
    }
    Ok(())
}

/// Returns the number of pages in a PDF. On macOS this uses CoreGraphics; elsewhere it is unsupported.
pub fn pdf_page_count(path: &Path) -> Result<u32, MediaError> {
    #[cfg(target_os = "macos")]
    {
        native::page_count(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(MediaError::Unsupported(
            "PDF rendering requires macOS CoreGraphics".into(),
        ))
    }
}

/// Renders a 1-based PDF page into an aspect-preserving RGBA frame bounded by `max_w` and `max_h`.
pub fn render_pdf_page(
    path: &Path,
    page: u32,
    max_w: u32,
    max_h: u32,
) -> Result<Frame, MediaError> {
    #[cfg(target_os = "macos")]
    {
        native::render(path, page, max_w, max_h)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, page, max_w, max_h);
        Err(MediaError::Unsupported(
            "PDF rendering requires macOS CoreGraphics".into(),
        ))
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn test_two_page_pdf() -> Vec<u8> {
    let stream = |content: &str| {
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            content.len(),
            content
        )
    };
    let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] /CropBox [10 5 90 45] /Contents 4 0 R >>".to_string(),
            stream("1 0 0 rg 10 5 40 20 re f\n0 0 1 rg 50 20 30 25 re f\n"),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] /Rotate 90 /Contents 6 0 R >>".to_string(),
            stream("0 0.7 0 rg 10 10 70 30 re f\n"),
        ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    pdf
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_pdf_bounds_without_platform_dependencies() {
        assert!(validate_dimensions(1, 8192).is_ok());
        assert!(validate_dimensions(0, 1).is_err());
        assert!(validate_dimensions(8193, 1).is_err());
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.pdf");
        std::fs::write(&empty, b"").unwrap();
        assert!(validate_pdf_file(&empty).is_err());
        #[cfg(not(target_os = "macos"))]
        assert!(matches!(
            pdf_page_count(&empty),
            Err(MediaError::Unsupported(_))
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn renders_vector_pages_with_crop_and_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.pdf");
        std::fs::write(&path, test_two_page_pdf()).unwrap();
        assert_eq!(pdf_page_count(&path).unwrap(), 2);
        let first = render_pdf_page(&path, 1, 80, 80).unwrap();
        assert_eq!((first.width, first.height), (80, 40));
        let px: std::collections::HashSet<_> = first
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[0], p[1], p[2]))
            .collect();
        assert!(
            px.len() > 2,
            "vector fills should produce non-uniform pixels"
        );
        let pixel = |x: usize, y: usize| {
            let at = (y * first.width as usize + x) * 4;
            &first.rgba[at..at + 3]
        };
        assert_eq!(
            pixel(60, 5),
            &[0, 0, 255],
            "row 0 should show the visual top"
        );
        assert_eq!(
            pixel(10, 35),
            &[255, 0, 0],
            "lower-left fill should appear near the bottom"
        );
        // Larger than the 80×40 pt page: content must scale up, not sit at 1:1.
        let big = render_pdf_page(&path, 1, 320, 320).unwrap();
        assert_eq!((big.width, big.height), (320, 160));
        let big_pixel = |x: usize, y: usize| {
            let at = (y * big.width as usize + x) * 4;
            &big.rgba[at..at + 3]
        };
        assert_eq!(big_pixel(240, 20), &[0, 0, 255]);
        assert_eq!(big_pixel(40, 140), &[255, 0, 0]);
        let rotated = render_pdf_page(&path, 2, 80, 80).unwrap();
        assert_eq!((rotated.width, rotated.height), (40, 80));
        assert!(render_pdf_page(&path, 0, 80, 80).is_err());
        assert!(render_pdf_page(&path, 3, 80, 80).is_err());
        assert!(pdf_page_count(&dir.path().join("missing.pdf")).is_err());
        let garbage = dir.path().join("garbage.pdf");
        std::fs::write(&garbage, b"not a PDF").unwrap();
        assert!(pdf_page_count(&garbage).is_err());
        assert!(render_pdf_page(&path, 3, 80, 80).is_err());
        assert!(pdf_page_count(&dir.path().join("missing.pdf")).is_err());
        let garbage = dir.path().join("garbage.pdf");
        std::fs::write(&garbage, b"not a PDF").unwrap();
        assert!(pdf_page_count(&garbage).is_err());
    }
}
