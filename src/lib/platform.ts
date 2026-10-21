// vitest(node) 에서는 navigator 가 없거나 Mac 이 아니므로 Windows 표기로 테스트된다
export const isMac = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
