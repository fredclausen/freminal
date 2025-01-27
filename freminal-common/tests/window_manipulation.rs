use freminal_common::window_manipulation::WindowManipulation;
// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
use test_log::test;

#[test]
fn test_window_manipulation_try_from() {
    let window_manipulation = WindowManipulation::try_from((1, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::DeIconifyWindow);

    let window_manipulation = WindowManipulation::try_from((2, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::MinimizeWindow);

    let window_manipulation = WindowManipulation::try_from((3, 10, 20)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::MoveWindow(10, 20));

    let window_manipulation = WindowManipulation::try_from((4, 30, 40)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ResizeWindow(30, 40)
    );

    let window_manipulation = WindowManipulation::try_from((5, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::RaiseWindowToTopOfStackingOrder
    );

    let window_manipulation = WindowManipulation::try_from((6, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::LowerWindowToBottomOfStackingOrder
    );

    let window_manipulation = WindowManipulation::try_from((7, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::RefreshWindow);

    let window_manipulation = WindowManipulation::try_from((8, 50, 60)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ResizeWindowToLinesAndColumns(50, 60)
    );

    let window_manipulation = WindowManipulation::try_from((9, 1, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::MaximizeWindow);

    let window_manipulation = WindowManipulation::try_from((9, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::RestoreNonMaximizedWindow
    );

    let window_manipulation = WindowManipulation::try_from((10, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::NotFullScreen);

    let window_manipulation = WindowManipulation::try_from((10, 1, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::FullScreen);

    let window_manipulation = WindowManipulation::try_from((10, 2, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::ToggleFullScreen);

    let window_manipulation = WindowManipulation::try_from((11, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::ReportWindowState);

    let window_manipulation = WindowManipulation::try_from((13, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowPositionWholeWindow
    );

    let window_manipulation = WindowManipulation::try_from((13, 1, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowPositionWholeWindow
    );

    let window_manipulation = WindowManipulation::try_from((13, 2, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowPositionTextArea
    );

    let window_manipulation = WindowManipulation::try_from((14, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowSizeInPixels
    );

    let window_manipulation = WindowManipulation::try_from((14, 1, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowSizeInPixels
    );

    let window_manipulation = WindowManipulation::try_from((14, 2, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportWindowTextAreaSizeInPixels
    );

    let window_manipulation = WindowManipulation::try_from((15, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportRootWindowSizeInPixels
    );

    let window_manipulation = WindowManipulation::try_from((16, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportCharacterSizeInPixels
    );

    let window_manipulation = WindowManipulation::try_from((18, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportTerminalSizeInCharacters
    );

    let window_manipulation = WindowManipulation::try_from((19, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::ReportRootWindowSizeInCharacters
    );

    let window_manipulation = WindowManipulation::try_from((20, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::ReportIconLabel);

    let window_manipulation = WindowManipulation::try_from((21, 0, 0)).unwrap();
    assert_eq!(window_manipulation, WindowManipulation::ReportTitle);

    let window_manipulation = WindowManipulation::try_from((22, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SaveWindowTitleToStack
    );

    let window_manipulation = WindowManipulation::try_from((22, 1, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SaveWindowTitleToStack
    );

    let window_manipulation = WindowManipulation::try_from((22, 2, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SaveWindowTitleToStack
    );

    let window_manipulation = WindowManipulation::try_from((23, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::RestoreWindowTitleFromStack
    );

    let window_manipulation = WindowManipulation::try_from((23, 1, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::RestoreWindowTitleFromStack
    );

    let window_manipulation = WindowManipulation::try_from((23, 2, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::RestoreWindowTitleFromStack
    );

    let window_manipulation = WindowManipulation::try_from((24, 0, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SetTitleBarText(String::new())
    );

    let window_manipulation = WindowManipulation::try_from((24, 1, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SetTitleBarText(String::new())
    );

    let window_manipulation = WindowManipulation::try_from((24, 2, 0)).unwrap();
    assert_eq!(
        window_manipulation,
        WindowManipulation::SetTitleBarText(String::new())
    );

    let window_manipulation = WindowManipulation::try_from((24, 3, 0));
    assert!(window_manipulation.is_err());
}
