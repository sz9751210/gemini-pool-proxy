import { Link } from "react-router-dom";

export function NotFoundPage() {
  return (
    <div className="empty-page">
      <h2>找不到頁面</h2>
      <p>此路由不存在或尚未實作。</p>
      <Link to="/">回到登入頁</Link>
    </div>
  );
}
