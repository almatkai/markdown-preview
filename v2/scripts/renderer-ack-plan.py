"""Required host-owned Markdown code-copy feedback fix; emits patch, never edits host/fixture."""
from pathlib import Path
import difflib
v=Path(__file__).resolve().parents[1];path='src/renderer/ui/Markdown.tsx'
source=(v/'fixtures/beta'/path).read_text()
helper='''// Native object acknowledgements must be positive and non-contradictory.
function acknowledgedClipboardWrite(value: unknown): boolean {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return false
  const reply = value as { ok?: unknown; error?: unknown; success?: unknown }
  return reply.ok === true &&
    (reply.error === undefined || reply.error === null) &&
    (reply.success === undefined || reply.success === true)
}

'''
needle='function MarkdownView('
assert source.count(needle)==1
result='// Modified from pinned beta: strict native code-copy acknowledgement feedback.\n'+source.replace(needle,helper+needle)
needle="(result) => showResult(result && result.ok === false ? 'Failed' : 'Copied'),"
assert result.count(needle)==1
result=result.replace(needle,"(result) => showResult(acknowledgedClipboardWrite(result) ? 'Copied' : 'Failed'),")
(v/'integration/Markdown.tsx').write_text(result)
(v/'integration/markdown-copy-ack.patch').write_text(''.join(difflib.unified_diff(source.splitlines(True),result.splitlines(True),fromfile='a/'+path,tofile='b/'+path)))
print('Prepared required host Markdown code-copy ack patch; original fixture/host untouched')
