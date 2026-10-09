// 设置页的数据形状。
//
// 放在独立模块里，是因为 <script setup> 不能导出类型 —— 对话框和它的子对话框都要用。

/** 背景图像设置。 */
export interface BackgroundPrefs {
  path: string;
  opacity: number;
  /** tile | center | fit | stretch */
  placement: string;
  /** 只对这个项目生效（否则全局）。 */
  projectOnly: boolean;
}

/** 设置页能改的全部内容。 */
export interface SettingsDraft {
  skin: "vscode" | "jb";
  density: string;
  left: boolean;
  right: boolean;
  leftWidth: number;
  rightWidth: number;
  tabView: "table" | "kv" | "json";
  background: BackgroundPrefs;
}
