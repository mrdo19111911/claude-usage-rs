# So sánh script Inno Setup của 4 dự án mã nguồn mở (cho bộ cài claude-usage.exe)

Mục tiêu: cài theo người dùng, không cần quyền admin, vào `{localappdata}\Programs`. Bộ cài cần đóng sạch app trên khay trước khi nâng cấp hoặc gỡ, và có tùy chọn tự chạy cùng Windows qua HKCU Run.
Đã đọc trực tiếp file `.iss` của từng dự án (tháng 10/2026).

| Dự án | Phương pháp đóng app | Quyền / thư mục | Tự chạy cùng Windows | Mức phù hợp | Chi phí |
|---|---|---|---|---|---|
| Zed (Rust) | `AppMutex={code:GetAppMutex}` (mutex rỗng khi tự cập nhật im lặng) và `CloseApplications=force` (Restart Manager); không taskkill | `PrivilegesRequired=lowest`, `DefaultDirName={autopf}\{#AppName}`, x64compatible | Không có | Cao về khung [Setup]; phần cập nhật/Appx thì thừa | Thấp nếu chỉ chép [Setup] |
| espanso (Rust, khay) | Không đóng lúc cài; lúc gỡ dùng `[UninstallRun]` chạy `taskkill /im espansod.exe /f /t` | `PrivilegesRequired=lowest`, `{autopf}`, `lzma`, `WizardStyle=modern` | Không do bộ cài làm (app tự đăng ký) | Trung bình: gọn, nhưng kill cứng dễ hỏng dữ liệu đang ghi | Rất thấp |
| ShareX (C#, khay) | `AppMutex={#MyAppId}`; `InitializeUninstall` gọi `CheckForMutexes`, hỏi người dùng rồi `taskkill.exe /f /im` | `PrivilegesRequired=none`, `{commonpf}` (cài cho máy) | Task `CreateStartupIcon`, shortcut vào `{userstartup}` với `-silent` | Trung bình: mẫu [Code] gỡ cài tốt, quyền/thư mục không hợp | Thấp |
| Greenshot (C#, khay) | Chỉ `AppMutex=F48E86D3-…` (Setup/Uninstall báo app đang chạy, bắt người dùng tự đóng) | `PrivilegesRequired=lowest` + `PrivilegesRequiredOverridesAllowed=dialog`, `{autopf}` | Task `startup` → `[Registry] Root: HKA; …\CurrentVersion\Run; Flags: uninsdeletevalue; Tasks: startup` | Cao về phần tự chạy | Thấp |

Điểm chung: không dự án nào xóa dữ liệu người dùng khi gỡ. Zed chỉ xóa `{app}\updates`, `{app}\tools`…; Greenshot chỉ xóa thư mục `{app}` rỗng.
Cả 4 đều có `[Run] … Flags: nowait postinstall` (espanso thêm `skipifsilent`, Zed dùng `Check: WizardNotSilent`).

Trích dẫn đáng chú ý:
- Zed: `ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`, `Compression=lzma`, `SolidCompression=yes`, `UninstallDisplayIcon={app}\{#AppExeName}.exe`, `WizardStyle=modern`.
- Greenshot: `Compression=lzma2/ultra64`, `SetupIconFile=..\..\src\Greenshot\icons\applicationIcon\icon.ico`; `[Run]` có thêm `runasoriginaluser`.
- ShareX: tham số `/UPDATE` ẩn bớt task khi nâng cấp, `/NORUN` bỏ bước chạy app sau khi cài.
- Tài liệu Inno: `AppMutex` được kiểm tra ở cả Setup lẫn Uninstall. `CloseApplications` (mặc định `yes`) dùng Restart Manager và chỉ áp dụng khi cài, cho file trong [Files]/[InstallDelete].

## Nguồn
- https://github.com/zed-industries/zed/blob/main/crates/zed/resources/windows/zed.iss
- https://github.com/espanso/espanso/blob/dev/scripts/resources/windows/setupscript.iss
- https://github.com/ShareX/ShareX/blob/develop/ShareX.Setup/InnoSetup/ShareX-setup.iss
- https://github.com/greenshot/greenshot/blob/release/1.3/installer/innosetup/setup.iss
- https://jrsoftware.org/ishelp/topic_setup_appmutex.htm , https://jrsoftware.org/ishelp/topic_setup_closeapplications.htm

## Chọn
Lấy khung [Setup] của Zed, phần tự chạy của Greenshot (đổi HKA thành HKCU) và mẫu [Code] của ShareX, nhưng thay `taskkill /f` bằng `claude-usage.exe --quit`:

- `PrivilegesRequired=lowest` và **không** đặt `PrivilegesRequiredOverridesAllowed`, để không bao giờ hỏi quyền admin.
- `DefaultDirName={localappdata}\Programs\Claude Usage`. Hằng `{autopf}` cũng trỏ đúng chỗ này khi cài ở chế độ lowest. Thêm `DisableDirPage=yes`, `DisableProgramGroupPage=yes`.
- `AppId={{GUID cố định}}`: không bao giờ đổi, vì nâng cấp nhận ra bản cũ nhờ nó.
- `ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`, `Compression=lzma2/ultra64`, `SolidCompression=yes`, `WizardStyle=modern`.
- `SetupIconFile=` trỏ tới icon của app, `UninstallDisplayIcon={app}\claude-usage.exe`.
- `AppMutex=claude-usage-dashboard`: tên không tiền tố tương đương `Local\` trong phiên hiện tại. Đây là lưới an toàn cuối: nếu `--quit` thất bại, Inno báo app đang chạy.
- `CloseApplications=no`. App khay chỉ có cửa sổ ẩn, Restart Manager không đóng sạch được, và nó cũng không chạy lúc gỡ.
- `[Code]`:
  - `PrepareToInstall`: nếu `FileExists(ExpandConstant('{app}\claude-usage.exe'))` thì `Exec(…, '--quit', …, SW_HIDE, ewWaitUntilTerminated, rc)`.
  - `InitializeUninstall`: làm như trên với `{app}`, trả `True`.
  - Không dùng taskkill, trừ khi muốn có phương án cuối sau khi `--quit` hết thời gian chờ.
- Task `Name: autostart; Description: "Khởi động cùng Windows"; Flags: unchecked`.
  - `[Registry] Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ClaudeUsage"; ValueData: """{app}\claude-usage.exe"""; Flags: uninsdeletevalue; Tasks: autostart`.
  - Dùng HKCU Run thay shortcut Startup vì app có thể tự bật/tắt cùng khóa đó.
- `[Icons] Name: "{autoprograms}\Claude Usage"; Filename: "{app}\claude-usage.exe"`.
- `[Run] Filename: "{app}\claude-usage.exe"; Description: "{cm:LaunchProgram,Claude Usage}"; Flags: nowait postinstall skipifsilent`.
- Không xóa `%USERPROFILE%\.claude-usage-dashboard` khi gỡ, như cả 4 dự án. Nếu cần, chỉ hỏi người dùng trong `CurUninstallStepChanged(usPostUninstall)` với mặc định là giữ.

## Cập nhật 12/11/2026: cài cho mọi user hay chỉ một user

Lý do: trên máy thử (Windows 10 22H2), Start không nhận shortcut mới trong thư mục Start Menu của user nữa, nhưng vẫn nhận shortcut mới trong thư mục chung (đã đo bằng shortcut thử). Cách so sánh: đọc trực tiếp file `.iss` của từng dự án và tài liệu Inno.

| Dự án | Phương pháp | Mức phù hợp | Chi phí |
|---|---|---|---|
| HeidiSQL (`out/heidisql.iss`) | `PrivilegesRequired=admin` + `PrivilegesRequiredOverridesAllowed=commandline dialog`, `DefaultDirName={autopf}\…` | Cao: mặc định cho mọi user, vẫn chọn được "chỉ mình tôi" | Thấp: vài dòng [Setup] |
| VS Code (`build/win32/code.iss`) | Hai bộ cài riêng: User (`lowest`) và System (`admin`) | Thấp: phải build và phát hành hai file | Cao |
| Git for Windows (`installer/install.iss`) | `[Run] … postinstall runasoriginaluser`, tách nhánh bằng `IsAdminInstallMode()` | Cao cho phần chạy app sau khi cài | Thấp |
| Greenshot | `lowest` + `dialog`, tự chạy qua `Root: HKA` | Cao cho phần tự chạy | Thấp |

Tài liệu Inno: `UsePreviousPrivileges` (mặc định `yes`) khiến bản nâng cấp giữ chế độ của bản đã cài và không hỏi lại. Cờ `runascurrentuser` là mặc định cho mục [Run] không có `postinstall`, nên mục `/RELAUNCH` phải ghi rõ `runasoriginaluser`.

**Chọn:**
- Khung của HeidiSQL: mặc định `admin`, có `dialog`, `{autopf}`.
- `Root: HKA` của Greenshot.
- `runasoriginaluser` của Git for Windows, đặt trên cả hai mục [Run].

Nguồn:
- https://github.com/HeidiSQL/HeidiSQL/blob/master/out/heidisql.iss
- https://github.com/microsoft/vscode/blob/main/build/win32/code.iss
- https://github.com/git-for-windows/build-extra/blob/main/installer/install.iss
- https://jrsoftware.org/ishelp/topic_setup_privilegesrequiredoverridesallowed.htm
- https://jrsoftware.org/ishelp/topic_setup_usepreviousprivileges.htm
- https://jrsoftware.org/ishelp/topic_runsection.htm
