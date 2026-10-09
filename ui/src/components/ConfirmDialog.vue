<script setup lang="ts">
// 通用确认框（危险操作用）。一句话说清「会发生什么」，再给一个需要主动点的按钮。

defineProps<{
  title: string;
  message: string;
  detail?: string;
  confirmText?: string;
  danger?: boolean;
}>();
defineEmits<{ (e: "confirm"): void; (e: "cancel"): void }>();
</script>

<template>
  <div class="overlay" @click.self="$emit('cancel')">
    <div class="dialog">
      <h2>{{ title }}</h2>
      <p class="message">{{ message }}</p>
      <p v-if="detail" class="detail">{{ detail }}</p>
      <footer>
        <!-- @ts-expect-error vscode-button 是自定义元素 -->
        <vscode-button secondary @click="$emit('cancel')">取消</vscode-button>
        <!-- @ts-expect-error 自定义元素 -->
        <vscode-button @click="$emit('confirm')">{{ confirmText ?? "确定" }}</vscode-button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: fixed; inset: 0; background: rgba(6, 8, 12, 0.62); display: grid; place-items: center; z-index: 75; }

.dialog {
  width: 460px;
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: var(--r4);
  box-shadow: var(--shadow-pop);
  padding: var(--s6);
  display: flex;
  flex-direction: column;
  gap: var(--s4);
}

h2 { font-size: var(--fs-lg); }
.message { margin: 0; color: var(--text); font-size: var(--fs-sm); line-height: 1.7; }
.detail { margin: 0; color: var(--text-faint); font-size: var(--fs-xs); line-height: 1.7; }

footer { display: flex; justify-content: flex-end; gap: var(--s3); margin-top: var(--s2); }

.danger-solid { background: var(--err); border-color: var(--err); color: #fff; font-weight: 500; }
.danger-solid:hover:not(:disabled) { background: #ef5f63; border-color: #ef5f63; }
</style>
