// 发布版不弹控制台窗口；debug 版保留控制台方便看日志。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sc2trans_gui::run();
}
