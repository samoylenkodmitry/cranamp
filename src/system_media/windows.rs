#![allow(unsafe_code)]

use windows::{
    Media::SystemMediaTransportControls,
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
    Win32::{
        Foundation::HWND,
        System::WinRT::{
            ISystemMediaTransportControlsInterop, RoInitialize, RoUninitialize,
            RO_INIT_MULTITHREADED,
        },
    },
};

pub(super) fn prepare_artwork(bytes: &[u8]) -> windows::core::Result<RandomAccessStreamReference> {
    // This runs on Cranpose's blocking executor. Balance the worker's WinRT
    // initialization even when decoding or storing the thumbnail fails.
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }
    let _apartment = Apartment;
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(bytes)?;
    writer.StoreAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    RandomAccessStreamReference::CreateFromStream(&stream)
}

pub(super) fn set_artwork(
    hwnd: isize,
    artwork: Option<&RandomAccessStreamReference>,
) -> windows::core::Result<()> {
    // Souvlaki leaves the Windows thumbnail unchanged when cover_url is None,
    // so explicitly set or clear our already prepared stream before Update.
    // The handle belongs to the live window held by our desktop integration.
    let interop: ISystemMediaTransportControlsInterop =
        windows::core::factory::<SystemMediaTransportControls, _>()?;
    let controls: SystemMediaTransportControls =
        unsafe { interop.GetForWindow(HWND(hwnd as *mut _)) }?;
    controls.DisplayUpdater()?.SetThumbnail(artwork)
}
