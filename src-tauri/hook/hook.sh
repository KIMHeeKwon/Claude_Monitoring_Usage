#!/bin/sh
# Claude Usage 모니터 statusline 훅.
# stdin으로 오는 Claude Code statusline JSON을 status.json에 저장하고,
# 원래 있던 statusline 명령이 있으면 같은 입력을 그대로 넘겨 화면 출력은 그대로 둔다.
DIR=${CLAUDE_USAGE_DIR:-"$HOME/.claude/usage-monitor"}
IN=$(cat)
printf '%s' "$IN" > "$DIR/status.json.tmp" 2>/dev/null && mv -f "$DIR/status.json.tmp" "$DIR/status.json" 2>/dev/null
if [ -s "$DIR/next-command" ]; then
  printf '%s' "$IN" | sh -c "$(cat "$DIR/next-command")"
fi
