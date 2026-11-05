/**
 * Thư mục câu hỏi dạng cây → danh sách phẳng có độ sâu và đường dẫn ("Chương 1 / Bài 2"), theo thứ tự cây.
 * @param {Array<{ id: string, parentId: string | null, name: string, questionCount: number }>} folders
 */
export function flattenFolders(folders) {
  const children = new Map();
  for (const f of folders) {
    const key = f.parentId ?? null;
    if (!children.has(key)) children.set(key, []);
    children.get(key).push(f);
  }
  const out = [];
  const visit = (parentId, depth, prefix) => {
    for (const f of children.get(parentId) ?? []) {
      const path = prefix ? `${prefix} / ${f.name}` : f.name;
      out.push({ ...f, depth, path });
      visit(f.id, depth + 1, path);
    }
  };
  visit(null, 0, '');
  // Thư mục mồ côi (cha không còn) vẫn hiện ở gốc.
  const seen = new Set(out.map((f) => f.id));
  for (const f of folders) if (!seen.has(f.id)) out.push({ ...f, depth: 0, path: f.name });
  return out;
}
