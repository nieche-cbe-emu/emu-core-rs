
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::machine::{ApiFn, Emu, Mach};

pub mod aps;
pub mod oldlib;
pub mod dfio;
pub mod fileio;
pub mod dfpkg;
pub mod gfx_api;
pub mod mem;
pub mod misc;
pub mod misc2;
pub mod misc3;
pub mod misc4;
pub mod printf;
pub mod screen;
pub mod storage;
pub mod text;
pub mod sysmisc;
pub mod std_c;

pub const MEM_T: &str = "VmMemoryManagerTag";
pub const STD_T: &str = "VmStdManagerTag";
pub const LCD_T: &str = "VmLcdManagerTag";
pub const IO_T: &str = "VmIoManagerTag";
pub const TIME_T: &str = "VmTimeManagerTag";
pub const BILL_T: &str = "VmBillingManagerTag";
pub const GAME_T: &str = "GameManagerOldTag";
pub const SYS_T: &str = "VmSysManagerTag";
pub const UCS_T: &str = "VmUcs2StrManagerTag";
pub const SCR_T: &str = "VmScreenManagerTag";
pub const UTIL_T: &str = "VmGameUtilManagerTag";
pub const DFE_T: &str = "VmDFEnginelManagerTag";
pub const GLCD_T: &str = "VmGameLcdManagerTag";
pub const CTRL_T: &str = "VmCtrlManagerTag";
pub const AUD_T: &str = "VmAudioManagerTag";

pub struct Entry {
    pub tag: Option<&'static str>,
    pub name: &'static str,
    pub func: ApiFn,
}

macro_rules! e {
    ($tag:expr, $name:expr, $f:path) => {
        Entry {
            tag: Some($tag),
            name: $name,
            func: $f,
        }
    };
}

pub static ENTRIES: &[Entry] = &[

    e!(MEM_T, "dF_Malloc_In", mem::df_malloc_in),
    e!(MEM_T, "dF_Malloc_debug", mem::df_malloc_in),
    e!(MEM_T, "dF_Free", mem::df_free),
    e!(MEM_T, "mallocBigMen", mem::malloc_big),
    e!(MEM_T, "mallocBigMen2", mem::malloc_big),
    e!(MEM_T, "mallocSysMem", mem::malloc_big),
    e!(MEM_T, "mallocBigMen_debug", mem::malloc_big),
    e!(MEM_T, "freeBigMen", mem::free_big),
    e!(MEM_T, "freeBigMen2", mem::free_big),
    e!(MEM_T, "freeSysMen", mem::free_big),
    e!(MEM_T, "RemallocBigMen", mem::remalloc_big),
    e!(MEM_T, "mF_GetGMemoryBlockPtr", mem::get_gblock),
    e!(MEM_T, "mF_MallocGmemoryBlock", mem::gblock_malloc),
    e!(MEM_T, "MB_Malloc", mem::gblock_malloc),
    e!(MEM_T, "dF_InitMemory", mem::noop),
    e!(MEM_T, "dF_InitMemoryEx", mem::noop),
    e!(MEM_T, "mF_InitMemoryBlock", mem::noop),
    e!(MEM_T, "mF_InitGmemoryBlock", mem::noop),
    e!(MEM_T, "dF_ReleaseMemory", mem::noop),
    e!(MEM_T, "mF_ReleaseGmemoryBlock", mem::noop),
    e!(MEM_T, "mF_resetGmemoryBlock", mem::noop),
    e!(MEM_T, "mF_MemoryBlock_Reset", mem::noop),
    e!(MEM_T, "mF_MemoryBlock_Release", mem::noop),
    e!(MEM_T, "dF_Memory_gc", mem::noop),

    e!(LCD_T, "VMDrawImageWithClipEx", gfx_api::draw_img_clip_ex),
    e!(LCD_T, "VMDrawImageClipAndAlphaEx", gfx_api::draw_img_clip_alpha_ex),
    e!(LCD_T, "VMFillRect", gfx_api::fill_rect),
    e!(LCD_T, "VMFillRectEx", gfx_api::fill_rect_ex),
    e!(LCD_T, "IMG_CreateImageFormStream", gfx_api::img_from_stream),
    e!(LCD_T, "ReleaseImage", gfx_api::release_image),
    e!(GAME_T, "IMG_Destory", gfx_api::release_image),

    e!(SCR_T, "vmAddScreen", screen::add_screen),
    e!(SCR_T, "vmAddScreenEx", screen::add_screen_ex),
    e!(SCR_T, "vmScreenLoadResource", screen::screen_load_res),
    e!(GAME_T, "GAME_isKeyDown", screen::is_key_down),
    e!(GAME_T, "GAME_isKeyHold", screen::is_key_hold),
    e!(GAME_T, "Get_CurKeyDownState", screen::cur_key_state),
    e!(GAME_T, "SCREEN_IsPointerDown", misc::pointer_down),
    e!(GAME_T, "SCREEN_IsPointerUp", misc::pointer_up),
    e!(GAME_T, "SCREEN_IsPointerHold", misc::pointer_hold),
    e!(GAME_T, "SCREEN_IsPointerDrag", misc::pointer_drag),

    e!(BILL_T, "BILLING_GetPayNumByAppId", sysmisc::billing_paynum),
    e!(BILL_T, "Billing_SendSpecSms", sysmisc::billing_send_sms),
    e!(BILL_T, "BILLING_Pay", sysmisc::billing_pay),
    e!(BILL_T, "BILLING_PayMoreTimes", sysmisc::billing_pay),
    e!(BILL_T, "BILLING_Pay2", sysmisc::billing_pay),
    e!(BILL_T, "BILLING_Pay3", sysmisc::billing_pay),
    e!(BILL_T, "BILLING_PayForCBB", sysmisc::billing_pay_cbb),
    e!(BILL_T, "BILLING_PayForCBB3", sysmisc::billing_pay_cbb),
    e!(BILL_T, "BILLING_PayForPwd", sysmisc::billing_pay_pwd),
    e!(BILL_T, "BILLING_IsRegisterBillingInfo", sysmisc::billing_is_registered),
    e!(BILL_T, "BILLING_RegisterBillingInfo", sysmisc::billing_register_info),
    e!(BILL_T, "BILLING_SetBillingStatus", sysmisc::billing_set_status),
    e!(BILL_T, "BILLING_GetBillingStatus", sysmisc::billing_get_status),
    e!(BILL_T, "Billing_GetAppUsedStatus", sysmisc::billing_get_used),
    e!(BILL_T, "Billing_SetAppUsedStatus", sysmisc::billing_set_used),
    e!(BILL_T, "BILLING_CleanAppMonthBillInfo", sysmisc::billing_clean_month),
    e!(BILL_T, "BILLING_IsInTryStatus", misc::one),
    e!(BILL_T, "BILLING_GetTryDay", misc::one),
    e!(BILL_T, "BILLING_GetValidDayByAppId", sysmisc::billing_valid_day),
    e!(BILL_T, "BILLING_GetSmsNum", sysmisc::billing_pay_times),
    e!(BILL_T, "CDownGetFileNameByAppID", sysmisc::billing_file_name),
    e!(BILL_T, "Billing_CancelSms", sysmisc::billing_cancel_sms),
    e!(BILL_T, "BILLING_IsNeedPay", misc::zero),
    e!(BILL_T, "CDownIsMonthApp", misc::zero),
    e!(BILL_T, "BIllING_OpenBillingPromptWin", misc::zero),
    e!(BILL_T, "BILLING_GetBillSmsAddr", misc::zero),
    e!(BILL_T, "BILLING_GetBillSmsSuf", misc::zero),
    e!(BILL_T, "BILLING_GetPayTipContent", misc::zero),
    e!(BILL_T, "BILLING_CDownOption8", misc::zero),
    e!(BILL_T, "BILLING_CDownOption9", misc::zero),
    e!(BILL_T, "BILLING_CDownOption10", misc::zero),
    e!(BILL_T, "BILLING_IsWPay", misc::zero),
    e!(BILL_T, "BILLING_NewMonthPay", misc::zero),
    e!(BILL_T, "BILLING_NewMonthCancel", misc::zero),
    e!(BILL_T, "BILLING_SendRegisterSms", misc::zero),
    e!(BILL_T, "BILLING_Register", misc::zero),
    e!(BILL_T, "BILLING_GetRemainDay", sysmisc::billing_remain_day),
    e!(GAME_T, "BILLING_GetRemainDay", sysmisc::billing_remain_day),
    e!(SYS_T, "GetCoolBarKernelCurrentVersion", sysmisc::kernel_ver),
    e!(SYS_T, "VMGetOperator", sysmisc::zero),
    e!(SYS_T, "vMGetKeyNum", sysmisc::zero),
    e!(SYS_T, "VmGetScreenWidth", sysmisc::screen_w),
    e!(SYS_T, "VmGetScreenHeight", sysmisc::screen_h),
    e!(GAME_T, "GetScreenWidth", sysmisc::screen_w),
    e!(GAME_T, "GetScreenHeight", sysmisc::screen_h),
    e!(TIME_T, "VMGetTotalSeconds", sysmisc::total_seconds),
    e!(LCD_T, "VMGetCurrMainScreenImage", sysmisc::main_screen_image),
    e!(GAME_T, "initMemoryBlock", sysmisc::init_memory_block),
    e!(GAME_T, "initDreamFactoryEngine", sysmisc::zero),
    e!(GAME_T, "SCREEN_ChangeScreen", sysmisc::screen_change),
    e!(GAME_T, "DF_SetDataPackage", sysmisc::df_set_pkg),

    e!(GAME_T, "initDFDataPackage", dfpkg::init_df_datapackage),
    e!(DFE_T, "initDFDataPackage", dfpkg::init_df_datapackage),
    e!(GAME_T, "DF_GetResourceIDByFileName", dfpkg::df_res_id_by_name),
    e!(UTIL_T, "DF_GetResourceIDByFileName", dfpkg::df_res_id_by_name),
    e!(GAME_T, "DF_GetResourceByFileName", dfpkg::df_res_by_name),
    e!(UTIL_T, "DF_GetResourceByFileName", dfpkg::df_res_by_name),
    e!(GAME_T, "DF_GetTResource", dfpkg::df_res_by_name),
    e!(UTIL_T, "DF_GetTResource", dfpkg::df_res_by_name),
    e!(GAME_T, "DF_GetStreamTResource", dfpkg::df_res_by_name),
    e!(UTIL_T, "DF_GetStreamTResource", dfpkg::df_res_by_name),
    e!(GAME_T, "DF_GetResourceByResourceID", dfpkg::df_res_by_id),
    e!(UTIL_T, "DF_GetResourceByResourceID", dfpkg::df_res_by_id),

    e!(GAME_T, "DF_ReadShort", dfio::read_short),
    e!(UTIL_T, "DF_ReadShort", dfio::read_short),
    e!(UTIL_T, "ReadShort", dfio::read_short),
    e!(GAME_T, "DF_ReadInt", dfio::read_int),
    e!(UTIL_T, "DF_ReadInt", dfio::read_int),
    e!(GAME_T, "DF_WriteShort", dfio::write_short),
    e!(UTIL_T, "DF_WriteShort", dfio::write_short),
    e!(GAME_T, "DF_WriteInt", dfio::write_int),
    e!(UTIL_T, "DF_WriteInt", dfio::write_int),
    e!(GAME_T, "DF_GetMemoryBlock", dfio::get_memblock),
    e!(UTIL_T, "DF_GetMemoryBlock", dfio::get_memblock),
    e!(LCD_T, "VMGetFontWidth", dfio::font_width),
    e!(LCD_T, "vMGetFontWidthEx", dfio::font_width),
    e!(LCD_T, "VMGetFontHeight", dfio::font_height),
    e!(LCD_T, "vMGetFontHeightEx", dfio::font_height),
    e!(SYS_T, "vMAssert", sysmisc::zero),
    e!(SYS_T, "VmEnterWinClose", sysmisc::enter_win_close),
    e!(SYS_T, "VmEnterWinOpen", sysmisc::zero),

    e!(STD_T, "sprintf", printf::sprintf),
    e!(STD_T, "vsprintf", printf::sprintf),
    e!(STD_T, "printf", printf::printf),
    e!(GAME_T, "sprintf", printf::sprintf),
    e!(GAME_T, "vsprintf", printf::sprintf),
    e!(UTIL_T, "Storage_Date", storage::storage_date),
    e!(GAME_T, "Storage_Date", storage::storage_date),

    e!(LCD_T, "VMGetStringWidth", text::string_width),
    e!(LCD_T, "vMGetStringWidthEx", text::string_width),
    e!(LCD_T, "VMGetStringHeight", text::string_height),
    e!(LCD_T, "vMGetStringHeightEx", text::string_height),
    e!(LCD_T, "VMGetCharWidth", text::char_width),
    e!(LCD_T, "VMDrawString", text::draw_string),
    e!(LCD_T, "VMDrawStringEx", text::draw_string_ex),
    e!(LCD_T, "vMShowStringEx", text::draw_string_ex),
    e!(LCD_T, "VMDrawStringClipAlign", text::draw_string_clip),
    e!(LCD_T, "VMDrawStringClip", text::draw_string_clip),
    e!(LCD_T, "vMDrawStringClipBorder", text::draw_string_clip),
    e!(LCD_T, "vMDrawStringClipAlignBorder", text::draw_string_clip),
    e!(LCD_T, "vMShowStringClipAlign", text::draw_string_clip),
    e!(LCD_T, "vMShowStringClip", text::draw_string_clip),

    e!(LCD_T, "VMIsBacklightOn", misc::one),
    e!(LCD_T, "VMGetCurrFontType", misc::one),
    e!(LCD_T, "VmGetIsNeedRefreshLcd", misc::one),
    e!(SYS_T, "vMGetGameWinState", misc::zero),
    e!(STD_T, "memcmp", misc::memcmp),
    e!(GAME_T, "memcmp", misc::memcmp),
    e!(GAME_T, "SCREEN_GetPointerX", misc::pointer_x),
    e!(GAME_T, "SCREEN_GetPointerY", misc::pointer_y),
    e!(GAME_T, "DF_Sin", misc::sin),
    e!(UTIL_T, "DF_Sin", misc::sin),
    e!(GAME_T, "DF_Cos", misc::cos),
    e!(UTIL_T, "DF_Cos", misc::cos),
    e!(GAME_T, "Sqrt", misc::sqrt),
    e!(UTIL_T, "Sqrt", misc::sqrt),
    e!(LCD_T, "vMSetFontSize", misc::zero),
    e!(LCD_T, "vMResetFontSize", misc::zero),
    e!(LCD_T, "VMSetCurrFontType", misc::zero),
    e!(LCD_T, "VmAllowBackLight", misc::zero),
    e!(LCD_T, "VMCtrlBacklight", misc::zero),
    e!(LCD_T, "VmSetIsNeedRefreshLcd", misc::zero),
    e!(LCD_T, "VmLCDInvalidateRectEnable", misc::zero),
    e!(LCD_T, "VmSetVideoIsNeedClosed", misc::zero),

    e!(LCD_T, "VMDrawImageEx", gfx_api::draw_img_ex),
    e!(LCD_T, "VMDrawImage", gfx_api::draw_img),
    e!(LCD_T, "VMDrawImageWithAlpha", gfx_api::draw_img_alpha),
    e!(LCD_T, "VMDrawImageWithClip2", gfx_api::draw_img_clip2),
    e!(LCD_T, "VMDrawImageClipAndAlpha2", gfx_api::draw_img_clip_alpha2),
    e!(LCD_T, "VMDrawImageWithClip", gfx_api::draw_img_with_clip),
    e!(LCD_T, "VMDrawImageClipAndAlpha", gfx_api::draw_img_clip_alpha),
    e!(GAME_T, "DF_ReadString", dfio::read_string),
    e!(UTIL_T, "DF_ReadString", dfio::read_string),
    e!(GAME_T, "DF_ReadStringEx", dfio::read_string),
    e!(UTIL_T, "DF_ReadStringEx", dfio::read_string),
    e!(GAME_T, "DF_File_ReadString", dfio::read_string),
    e!(UTIL_T, "DF_File_ReadString", dfio::read_string),
    e!(GAME_T, "DF_ReadString2", dfio::read_string2),
    e!(UTIL_T, "DF_ReadString2", dfio::read_string2),
    e!(GAME_T, "DF_File_ReadShort", dfio::read_short),
    e!(UTIL_T, "DF_File_ReadShort", dfio::read_short),
    e!(GAME_T, "DF_File_ReadInt", dfio::read_int),
    e!(UTIL_T, "DF_File_ReadInt", dfio::read_int),
    e!(GAME_T, "GetStreamDataFormRes", dfio::get_stream_data),
    e!(GLCD_T, "GetStreamDataFormRes", dfio::get_stream_data),
    e!(GLCD_T, "CreateImage", gfx_api::img_from_stream),
    e!(GAME_T, "IMG_CreateImageFormRes", gfx_api::img_from_res),
    e!(GAME_T, "SetClip", oldlib::set_clip),
    e!(GAME_T, "GetClip", oldlib::get_clip),
    e!(GAME_T, "CheckClip", oldlib::check_clip),
    e!(GAME_T, "IMG_GetHeight", oldlib::img_height),
    e!(GAME_T, "IMG_GetWidth", oldlib::img_width),
    e!(GAME_T, "ToRGB", oldlib::to_rgb),
    e!(GAME_T, "DrawImageWithClipEx", oldlib::draw_img_clip_ex),
    e!(GAME_T, "DrawImageClipAndAlphaEx", oldlib::draw_img_clip_alpha_ex),
    e!(GAME_T, "DrawImageWithClip", oldlib::draw_img_clip),
    e!(GAME_T, "DrawImageClipAndAlpha", oldlib::draw_img_clip_alpha),
    e!(GAME_T, "DrawFullScreen", oldlib::draw_full_screen),
    e!(GAME_T, "GetLCDBuffer", oldlib::get_lcd_buffer),
    e!(GAME_T, "FillRect", oldlib::fill_rect),
    e!(GAME_T, "DrawLineEx", oldlib::draw_line_ex),
    e!(GAME_T, "initDFPictureLibrary", oldlib::init_picture_library),
    e!(GAME_T, "initDFWindows", oldlib::init_repaint_panel),
    e!("VmDFEnginelManagerTag", "DF_SendMessage", oldlib::panel_invalidate),
    e!(GAME_T, "OldLib_094", mem::malloc_big),
    e!(GAME_T, "OldLib_098", mem::free_big),
    e!(GAME_T, "OldLib_09c", mem::malloc_big),
    e!(GAME_T, "OldLib_0a0", mem::free_big),
    e!(GAME_T, "OldLib_0b0", misc2::get_tick),
    e!(GAME_T, "OldLib_0b4", fileio::sys_sleep),
    e!(GAME_T, "OldLib_0a4", oldlib::time_word8),
    e!(GAME_T, "OldLib_0a8", oldlib::time_word4),
    e!(GAME_T, "OldLib_0ac", oldlib::time_word0),
    e!(GAME_T, "Abs", oldlib::lib_abs),
    e!(GAME_T, "Max", oldlib::lib_max),
    e!(GAME_T, "Min", oldlib::lib_min),
    e!(GAME_T, "Random", oldlib::lib_random),
    e!(GAME_T, "initDFActor", oldlib::init_df_actor),
    e!(GAME_T, "GetFontWidth", oldlib::get_font_width),
    e!(GAME_T, "DrawVLine", oldlib::draw_vline),
    e!(GAME_T, "DrawHLine", oldlib::draw_hline),
    e!(GAME_T, "DrawLine", oldlib::draw_line),
    e!(GAME_T, "DrawRect", oldlib::draw_rect),
    e!(GAME_T, "OldLib_064", oldlib::draw_string_rect_old),
    e!(GAME_T, "NumToZhiFei", oldlib::num_to_zhifei),
    e!(GAME_T, "GetFontWidth_char", oldlib::get_font_width_char),
    e!(GAME_T, "GetFontHeight", oldlib::get_font_height),
    e!(GAME_T, "GetFontHeight_char", oldlib::get_font_height),
    e!(GAME_T, "DrawStringEx", oldlib::draw_string_ex),
    e!(GAME_T, "DrawString", oldlib::draw_string),
    e!(GAME_T, "InitTextBox", oldlib::init_text_box),
    e!(GAME_T, "DrawUI", oldlib::draw_ui),
    e!(GAME_T, "DrawUIHorizontal", oldlib::draw_ui_horizontal),
    e!(GAME_T, "DrawUISingleRepeat", oldlib::draw_ui_single_repeat),
    e!(GAME_T, "DrawUIFourXRepeat", oldlib::draw_ui_four_x_repeat),
    e!(GAME_T, "OldLib_08c", oldlib::draw_ui_four_x_repeat),
    e!(GAME_T, "DrawImageNumberEx", oldlib::draw_image_number_ex),
    e!(GAME_T, "DrawNumber", oldlib::draw_number),
    e!(GAME_T, "RefresScreen", oldlib::refres_screen),

    e!(TIME_T, "VMGetTickCount", misc2::get_tick),
    e!(TIME_T, "VMGetCurrentTime", misc2::current_time),
    e!(LCD_T, "VM_InvalidateLcd", misc2::invalidate),
    e!(LCD_T, "vM_InvalidateLcdEx", misc2::invalidate),
    e!(LCD_T, "VMGetImageWidth", misc2::image_width),
    e!(LCD_T, "VMGetImageHeight", misc2::image_height),
    e!(LCD_T, "VMDrawRectEx", misc2::draw_rect_ex),
    e!(LCD_T, "VMDrawLineEx", misc2::draw_line_ex),
    e!(LCD_T, "vMFillRectWithImage", misc2::fill_rect_with_image),
    e!(LCD_T, "vMFillRectWithImageEx", misc2::fill_rect_with_image),
    e!(CTRL_T, "VmPubDrawSoftkeyBarEx", misc2::softkey_bar),
    e!(CTRL_T, "VmPubDrawSoftkeyBar", misc2::softkey_bar),
    e!(CTRL_T, "vmPubDrawWinTitleEx", misc2::win_title),
    e!(CTRL_T, "vmPubDrawWinTitle", misc2::win_title),
    e!(GAME_T, "DF_GetFormatString", misc2::format_string),
    e!(UTIL_T, "DF_GetFormatString", misc2::format_string),
    e!(UTIL_T, "formatString", misc2::format_string),
    e!(LCD_T, "VMDrawStringRect", text::draw_string_rect),
    e!(LCD_T, "vMShowStringRect", text::draw_string_rect),
    e!(LCD_T, "vMShowString", text::draw_string),
    e!(LCD_T, "vMDrawStringBorder", text::draw_string),
    e!(GAME_T, "DF_GetResourceNameByID", dfpkg::df_res_name_by_id),
    e!(UTIL_T, "DF_GetResourceNameByID", dfpkg::df_res_name_by_id),
    e!(AUD_T, "vMAudioSetVolume", misc3::audio_volume),
    e!(AUD_T, "vMAudioPlayForGame", misc3::audio_play),
    e!(AUD_T, "vMAudioPlayForApp", misc3::audio_play),

    e!(IO_T, "VmIoManager+0x80", storage::nv_read),
    e!(IO_T, "VmIoManager+0x11c", storage::nv_write),
    e!(IO_T, "VmIoManager+0x120", storage::nv_write_tail),
    e!(IO_T, "Vm_file_open", fileio::file_open),
    e!(IO_T, "Vm_file_close", fileio::file_close),
    e!(IO_T, "Vm_file_read", fileio::file_read),
    e!(IO_T, "Vm_file_write", fileio::file_write),
    e!(IO_T, "Vm_file_seek", fileio::file_seek),
    e!(IO_T, "Vm_file_tell", fileio::file_tell),
    e!(IO_T, "Vm_file_getfilesize", fileio::file_size),
    e!(IO_T, "Vm_file_exist", fileio::file_exist),
    e!(IO_T, "Vm_file_direxist", fileio::file_exist),
    e!(IO_T, "Vm_file_mkdir", fileio::file_mkdir),
    e!(IO_T, "Vm_file_rmdir", fileio::file_mkdir),
    e!(IO_T, "Vm_get_sdcardStatus", fileio::sdcard),
    e!(IO_T, "Vm_get_sdcardStatusEx", fileio::sdcard),
    e!(SYS_T, "GetCoolbarDirPath", fileio::coolbar_dir),
    e!(SYS_T, "VmGetCDownFileName", fileio::cdown_str),
    e!(SYS_T, "VmGetCDownAppUrl", fileio::cdown_str),
    e!(IO_T, "Vm_find_first", fileio::find_first),
    e!(IO_T, "Vm_find_next", fileio::find_next),
    e!(IO_T, "Vm_find_close", fileio::find_close),
    e!(IO_T, "Vm_file_delete", fileio::file_delete),
    e!(STD_T, "strcmp", fileio::strcmp),
    e!(GAME_T, "strcmp", fileio::strcmp),
    e!(TIME_T, "VMSysSleep", fileio::sys_sleep),
    e!(SCR_T, "vmDeleteScreen", fileio::delete_screen),
    e!(LCD_T, "VMDestoryImage", gfx_api::release_image),
    e!(LCD_T, "VMDrawRect", misc2::draw_rect),
    e!(TIME_T, "VMStartTimer", misc3::start_timer),
    e!(TIME_T, "VMStopTimer", misc3::stop_timer),
    e!(SCR_T, "vmIsScreenFocus", misc3::is_focus),
    e!(SYS_T, "VMGetIMEI", misc3::get_imei),
    e!(SYS_T, "vMGetIMEI", misc3::get_imei),
    e!(SYS_T, "vmDlGetCurrAppId", misc3::cur_appid),
    e!(SYS_T, "VMGetPrjVersion", misc3::prj_version),
    e!(SYS_T, "VmSetFPS", misc3::set_fps),
    e!(SYS_T, "VmGetPrjCustom", misc::zero),
    e!(SYS_T, "VmGetOperatorMCC", misc::zero),
    e!(SYS_T, "VmGetOperatorMNC", misc::zero),
    e!(LCD_T, "vmResGetTxtWithDataPackage", misc3::res_get_txt),
    e!(LCD_T, "VmResGetDefTxt", misc3::res_get_txt),
    e!(LCD_T, "vmResGetTxtForGame", misc3::res_get_txt),
    e!(LCD_T, "VMCreateImage", misc3::create_image),
    e!(LCD_T, "VMCreateImageFromInRes", misc3::create_image),
    e!("VmDlResourceManagerTag", "vmGetDataPackage", misc3::get_data_package),
    e!(AUD_T, "vMAduioGetState", misc3::audio_state),
    e!(AUD_T, "vMAudioStop", misc3::audio_stop),
    e!(IO_T, "Vm_get_freespace", misc3::free_space),
    e!(IO_T, "Vm_get_freespace_ex", misc3::free_space),
    e!(SYS_T, "VmIsInnerApp", misc::zero),
    e!(SYS_T, "VmGetPlatformType", misc::zero),
    e!(SYS_T, "VMGetPrjName", misc3::get_prj),
    e!(SYS_T, "VmGetSmsCenterNum", misc3::get_smsc),
    e!(SYS_T, "cDownGetCompanyEx", misc3::write_empty),
    e!(SYS_T, "CDownGetServicePhone", misc3::write_empty),
    e!(SYS_T, "CDownGetCompany", misc3::write_empty),
    e!(SYS_T, "vMAudioIsSupportInCb", misc3::audio_supported),
    e!(SYS_T, "VMGetOperator", misc::zero),
    e!(STD_T, "vMstrnicmp", misc3::strnicmp),
    e!(MEM_T, "mF_MemoryBlock_Malloc", misc3::block_malloc),
    e!(GAME_T, "SCREEN_NotifyLoadResource", misc3::screen_notify_loadres),
    e!(BILL_T, "BILLING_GetCdownOption5", misc::zero),
    e!("VmNetManagerTag", "CloseChannel", misc::one),
    e!("VmNetManagerTag", "CancelHttpConnect", misc::one),
    e!("VmNetManagerTag", "SetDeactiveFlag", misc::zero),
    e!("VmNetManagerTag", "GetHttpData", misc3::get_http),
    e!("VmNetManagerTag", "PostHttpData", misc3::post_http),
    e!("VmNetManagerTag", "GetHttpDataEx", misc3::get_http),
    e!("VmNetManagerTag", "OpenChannel", misc4::open_channel),
    e!("VmNetManagerTag", "OpenChannel2", misc4::open_channel),
    e!("VmNetManagerTag", "OpenQQChannel", misc4::open_channel),
    e!("VmNetManagerTag", "CloseChannel", misc4::close_channel),
    e!("VmNetManagerTag", "CancelHttpConnect", misc4::close_channel),
    e!("VmNetManagerTag", "SetDeactiveFlag", misc4::zero),
    e!("VmNetManagerTag", "VMGetLinkSetNum", misc4::zero),
    e!("VmNetManagerTag", "VMGetWapIndex", misc4::zero),
    e!("VmNetManagerTag", "VMGetNetIndex", misc4::zero),
    e!("VmImManagerTag", "reserver_func01", aps::im_sms),
    e!("VmImManagerTag", "reserver_func02", misc4::zero),
    e!("VmImManagerTag", "reserver_func03", aps::im_nv_read),
    e!("VmImManagerTag", "reserver_func04", aps::im_nv_write),
    e!("VmImManagerTag", "reserver_func05", misc4::zero),
    e!("VmImManagerTag", "reserver_func06", misc4::zero),
    e!("VmImManagerTag", "reserver_func07", aps::im_get_aps_manager),
    e!("VmImManagerTag", "reserver_func08", misc4::zero),
    e!("VmDlAppStoreManagerTag", "vmAppStoreGetPath", aps::aps_path),
    e!("VmDlAppStoreManagerTag", "vmGetRunAppFileSystem", aps::aps_get_dev),
    e!("VmDlAppStoreManagerTag", "vmSetRunAppFileSystem", aps::aps_set_dev),
    e!("VmDlAppStoreManagerTag", "vmGetAppNumByType", aps::aps_app_num),
    e!("VmDlAppStoreManagerTag", "vmGetPhoneSupportApType", aps::aps_support_type),
    e!("VmDlAppStoreManagerTag", "vmGetSecurityCodeEx", aps::aps_security_code),
    e!(SYS_T, "VmGetScreenSize", sysmisc::screen_type),
    e!(LCD_T, "IMG_InitDataPage", misc::zero),
    e!(LCD_T, "IMG_InitDataPageEx", misc::zero),
    e!(LCD_T, "IMG_InitInnerDataPageEx", misc::zero),
    e!(LCD_T, "IMG_InitDataPageTxt", misc::zero),
    e!(LCD_T, "IMG_ReleaseDataPage", misc::zero),
    e!(LCD_T, "vMImageDecoderRegImageCodecHandler", misc::zero),

    e!(UCS_T, "vmutStrlenUcs2", fileio::ucs2_strlen),
    e!(UCS_T, "VmutStrlenUcs2", fileio::ucs2_strlen),
    e!(UCS_T, "vmutStrcpyUcs2", fileio::ucs2_strcpy),
    e!(UCS_T, "VmutStrcpyUcs2", fileio::ucs2_strcpy),
    e!(UCS_T, "vmutStrcatUcs2", fileio::ucs2_strcat),
    e!(UCS_T, "VmutStrcatUcs2", fileio::ucs2_strcat),
    e!(UCS_T, "vmutStrncmpUcs2", fileio::ucs2_strncmp),
    e!(UCS_T, "VmutStrncmpUcs2", fileio::ucs2_strncmp),
    e!(UCS_T, "VmutExpandStrcpy", fileio::expand_strcpy),
    e!(LCD_T, "VMGB2UCS2", fileio::gb2ucs2),
    e!(LCD_T, "VMUCS2GB", fileio::ucs2gb),
    e!(LCD_T, "vMGetUcs2StringWidth", fileio::ucs2_width),
    e!(LCD_T, "VMGetUcs2StringWidth", fileio::ucs2_width),
    e!(LCD_T, "vMDrawUcs2String", fileio::draw_ucs2),
    e!(LCD_T, "VMDrawUcs2String", fileio::draw_ucs2),
    e!(LCD_T, "vMShowUcs2String", fileio::draw_ucs2),

    e!(STD_T, "memcpy", std_c::memcpy),
    e!(STD_T, "memmove", std_c::memcpy),
    e!(STD_T, "memset", std_c::memset),
    e!(STD_T, "strlen", std_c::strlen),
    e!(STD_T, "strcpy", std_c::strcpy),
    e!(STD_T, "strncpy", std_c::strncpy),
    e!(STD_T, "strcat", std_c::strcat),
    e!(STD_T, "atoi", std_c::atoi),
    e!(STD_T, "atol", std_c::atoi),
    e!(STD_T, "rand", std_c::rand),
    e!(GAME_T, "VmGetRand", std_c::rand),
    e!(GAME_T, "memcpy", std_c::memcpy),
    e!(GAME_T, "memset", std_c::memset),
    e!(GAME_T, "strlen", std_c::strlen),
    e!(GAME_T, "strcpy", std_c::strcpy),
    e!(GAME_T, "strncpy", std_c::strncpy),
    e!(GAME_T, "strcat", std_c::strcat),
    e!(GAME_T, "atol", std_c::atoi),

    e!(UTIL_T, "CdRectPoint", misc4::cd_rect_point),
    e!(GAME_T, "CdRectPoint", misc4::cd_rect_point),
    e!(UTIL_T, "CdRect", misc4::cd_rect),
    e!(GAME_T, "CdRect", misc4::cd_rect),
    e!(UTIL_T, "CdRectPoint2", misc4::cd_rect_point2),
    e!(GAME_T, "CdRectPoint2", misc4::cd_rect_point2),
    e!(UTIL_T, "CdRect2", misc4::cd_rect2),
    e!(GAME_T, "CdRect2", misc4::cd_rect2),
    e!(UTIL_T, "DF_Degree", misc4::df_degree),
    e!(GAME_T, "DF_Degree", misc4::df_degree),
    e!(UTIL_T, "DF_String_Equal", misc4::df_string_equal),
    e!(GAME_T, "DF_String_Equal", misc4::df_string_equal),
    e!(GAME_T, "SCREEN_IsKeyDown", screen::is_key_down),
    e!(GAME_T, "SCREEN_IsKeyHold", screen::is_key_hold),
    e!(GAME_T, "SCREEN_IsKeyUp", misc4::is_key_up),
    e!(GAME_T, "DF_GetDataPackage", misc4::df_get_data_package),
    e!(SCR_T, "vmChangeScreen", misc4::change_screen),
    e!(SCR_T, "vmChangeScreenEx", misc4::change_screen_ex),
    e!(SCR_T, "vmSCREEN_ChangeScreen", misc4::change_screen_ex),
    e!(SCR_T, "vmIsBottomScreen", misc4::is_bottom_screen),
    e!(LCD_T, "VMGetLCDBuffer", misc4::get_lcd_buffer),
    e!(LCD_T, "vMDrawUcs2StringRect", text::draw_ucs2_string_rect),
    e!(LCD_T, "vMDrawUcs2StringRectEx", text::draw_ucs2_string_rect),
    e!(STD_T, "strncmp", misc4::strncmp),
    e!(STD_T, "vMstricmp", misc4::stricmp),
    e!(UCS_T, "vmutStrcmpUcs2", misc4::ucs2_strcmp),
    e!(UCS_T, "VmutStrcmpUcs2", misc4::ucs2_strcmp),
    e!(SYS_T, "GetCoolBarFullPath", misc4::coolbar_full_path),
    e!(SYS_T, "VmSupportOpenCamera", misc4::zero),
    e!(SYS_T, "VmOpenCamera", misc4::zero),
    e!(SYS_T, "vmSysIsHaveNetWork", misc4::zero),
    e!(SYS_T, "vMIsSimReady", misc4::zero),
    e!(MEM_T, "getShareMemAlloced", misc4::zero),
    e!(AUD_T, "vMAudioPause", misc4::audio_pause),
    e!(AUD_T, "VmMp3PauseByStream", misc4::audio_pause),
    e!(AUD_T, "VmMp3PauseByFile", misc4::audio_pause),
    e!(AUD_T, "vMAudioResume", misc4::audio_resume),
    e!(AUD_T, "VmMp3ResumeByStream", misc4::audio_resume),
    e!(AUD_T, "VmMp3ResumeByFile", misc4::audio_resume),
    e!(AUD_T, "vMAudioPlayByData", misc4::audio_play_by_data),
    e!(AUD_T, "vMAudioPlayWithDataPackage", misc3::audio_play),
];

struct Index {
    exact: HashMap<(&'static str, &'static str), ApiFn>,
    wild: HashMap<&'static str, ApiFn>,

    by_name: HashMap<&'static str, Option<ApiFn>>,
}

fn index() -> &'static Index {
    static IDX: OnceLock<Index> = OnceLock::new();
    IDX.get_or_init(|| {
        let mut exact = HashMap::new();
        let mut wild = HashMap::new();
        let mut by_name: HashMap<&'static str, Option<ApiFn>> = HashMap::new();
        for en in ENTRIES {
            match en.tag {
                Some(t) => {
                    exact.insert((t, en.name), en.func);
                }
                None => {
                    wild.insert(en.name, en.func);
                }
            }
            by_name
                .entry(en.name)
                .and_modify(|slot| {

                    if !matches!(slot, Some(f) if std::ptr::fn_addr_eq(*f, en.func)) {
                        *slot = None;
                    }
                })
                .or_insert(Some(en.func));
        }
        Index {
            exact,
            wild,
            by_name,
        }
    })
}

pub fn lookup(tag: &str, name: &str) -> Option<ApiFn> {
    let ix = index();
    if let Some(f) = ix.exact.get(&(tag, name)) {
        return Some(*f);
    }
    if let Some(f) = ix.wild.get(tag) {
        return Some(*f);
    }
    match ix.by_name.get(name) {
        Some(Some(f)) => Some(*f),
        _ => None,
    }
}

pub fn implemented() -> usize {
    index().by_name.len()
}

pub(crate) fn fill(uc: &mut Emu, addr: u32, v: u8, n: u32) {
    const CHUNK: usize = 0x10000;
    let mut left = n as usize;
    let mut at = addr;
    let buf = vec![v; CHUNK.min(left.max(1))];
    while left > 0 {
        let k = left.min(buf.len());
        uc.write(at, &buf[..k]);
        at += k as u32;
        left -= k;
    }
}
