import { useState, useEffect } from "react";
import { runtimeBase, getAuthToken, apiClient } from "../lib/apiClient";

interface ModelItem {
  id: string;
  displayName?: string;
  owned_by?: string;
  family?: string;
}

function classifyModel(id: string): string {
  const lowerId = id.toLowerCase();
  if (lowerId.includes("gemini-3")) return "Gemini 3 Series";
  if (lowerId.includes("gemini-2.5")) return "Gemini 2.5 Series";
  if (lowerId.includes("gemini-2.0")) return "Gemini 2.0 Series";
  if (lowerId.includes("gemini")) return "Gemini Legacy & Spec";
  if (lowerId.includes("gemma")) return "Gemma Open Models";
  if (lowerId.includes("imagen")) return "Imagen (Image/Design)";
  if (lowerId.includes("veo")) return "Veo (Video Gen)";
  return "Other AI Models";
}

export function ApiTesterPage() {
  const [activeTab, setActiveTab] = useState<"native" | "proxy">("native");
  
  const [proxyLoading, setProxyLoading] = useState(false);
  const [proxyModels, setProxyModels] = useState<ModelItem[] | null>(null);
  const [proxyError, setProxyError] = useState<string>("");

  const [nativeKey, setNativeKey] = useState("");
  const [nativeLoading, setNativeLoading] = useState(false);
  const [nativeModels, setNativeModels] = useState<ModelItem[] | null>(null);
  const [nativeError, setNativeError] = useState<string>("");

  const [availableKeys, setAvailableKeys] = useState<string[]>([]);

  useEffect(() => {
    async function loadKeys() {
      try {
        const res = await apiClient.getAllKeys();
        const combined = Array.from(new Set([...(res.valid_keys || []), ...(res.invalid_keys || [])]));
        setAvailableKeys(combined);
      } catch (e) {
        console.error("無法載入配置金鑰:", e);
      }
    }
    loadKeys();
  }, []);

  const testProxy = async () => {
    setProxyLoading(true);
    setProxyError("");
    setProxyModels(null);
    try {
      const base = await runtimeBase();
      const token = getAuthToken();
      const headers = new Headers();
      if (token) headers.set("Authorization", `Bearer ${token}`);
      
      const res = await fetch(`${base}/v1/models`, { headers });
      if (!res.ok) {
        throw new Error(`HTTP Error ${res.status}: ${res.statusText}`);
      }
      const data = await res.json();
      setProxyModels(data.data || []);
    } catch (e: any) {
      setProxyError(e.message || "發生未知錯誤");
    } finally {
      setProxyLoading(false);
    }
  };

  const testNative = async () => {
    // 過濾掉可能因為 .env 解析錯誤而殘留的引號與中括號
    const cleanedKey = nativeKey.replace(/[\[\]"'\s]/g, "");
    
    if (!cleanedKey) {
      setNativeError("請先輸入您的 Google API Key (例如：AIzaSy...)");
      return;
    }
    setNativeLoading(true);
    setNativeError("");
    setNativeModels(null);
    try {
      const res = await fetch(`https://generativelanguage.googleapis.com/v1beta/models?key=${cleanedKey}`);
      if (!res.ok) {
        let errMsg = res.statusText;
        try {
           const errJson = await res.json();
           if (errJson?.error?.message) errMsg = errJson.error.message;
        } catch {}
        throw new Error(`連線失敗 ${res.status}: ${errMsg} (請確認金鑰是否正確填寫)`);
      }
      const data = await res.json();
      
      // Filter & Format
      const mapped = (data.models || [])
        .filter((m: any) => m.name)
        .map((m: any) => {
          const id = m.name.replace("models/", "");
          return {
            id,
            displayName: m.displayName || id,
            owned_by: "google",
            family: classifyModel(id)
          };
        })
        .sort((a: ModelItem, b: ModelItem) => a.displayName!.localeCompare(b.displayName!));
        
      setNativeModels(mapped);
    } catch (e: any) {
      setNativeError(e.message || "發生未知錯誤");
    } finally {
      setNativeLoading(false);
    }
  };

  // Group models by family for native view
  const groupedNativeModels = nativeModels?.reduce((acc, curr) => {
    const fam = curr.family || "Other";
    if (!acc[fam]) acc[fam] = [];
    acc[fam].push(curr);
    return acc;
  }, {} as Record<string, ModelItem[]>) || {};

  return (
    <div className="card" style={{ overflow: "hidden" }}>
      <div className="card-header" style={{ borderBottom: "1px solid #1e293b", paddingBottom: "0" }}>
        <h3 className="card-title">📡 API 測試與鑑權診斷工具</h3>
        <p className="card-subtitle" style={{ marginTop: "0.5rem", color: "#a5b4fc", fontSize: "0.9rem", marginBottom: "1.5rem" }}>
          這個獨立工具讓您在不受系統緩存干擾的情況下，即時驗證網路連線與金鑰授權狀態。
        </p>
        
        {/* Sleek Tab Navigation */}
        <div style={{ display: "flex", gap: "2rem" }}>
          <button 
            type="button"
            onClick={() => setActiveTab("native")}
            style={{ 
              background: "transparent", border: "none", color: activeTab === "native" ? "#c084fc" : "#64748b",
              borderBottom: activeTab === "native" ? "2px solid #c084fc" : "2px solid transparent",
              padding: "0.5rem 0.5rem 0.8rem", fontSize: "1rem", fontWeight: activeTab === "native" ? "600" : "400",
              cursor: "pointer", transition: "all 0.2s"
            }}
          >
            <i className="fas fa-google" style={{ marginRight: "0.5rem" }} /> 原生 API (Google Core)
          </button>
          <button 
            type="button"
            onClick={() => setActiveTab("proxy")}
            style={{ 
              background: "transparent", border: "none", color: activeTab === "proxy" ? "#38bdf8" : "#64748b",
              borderBottom: activeTab === "proxy" ? "2px solid #38bdf8" : "2px solid transparent",
              padding: "0.5rem 0.5rem 0.8rem", fontSize: "1rem", fontWeight: activeTab === "proxy" ? "600" : "400",
              cursor: "pointer", transition: "all 0.2s"
            }}
          >
            <i className="fas fa-server" style={{ marginRight: "0.5rem" }} /> 本機代理端 (Local Proxy)
          </button>
        </div>
      </div>

      <div className="card-body" style={{ minHeight: "60vh", background: "#0f172a" }}>
        
        {/* ----------------- 原生測試 TAB ----------------- */}
        {activeTab === "native" && (
          <div className="fade-in">
            <div style={{ background: "#1e293b", padding: "1.5rem", borderRadius: "10px", border: "1px solid #334155", marginBottom: "2rem" }}>
              <h4 style={{ marginBottom: "1rem", color: "#e879f9", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <i className="fas fa-key" /> 金鑰獨立分析與權限診斷
              </h4>
              <p style={{ marginBottom: "1.5rem", color: "#94a3b8", fontSize: "0.95rem", lineHeight: "1.5" }}>
                請填入或選擇一把 API 金鑰，系統會直接呼叫 Google 原生介面取得該金鑰的「授權模型存取清單」。
                這是排除錯誤連線與確認新實驗模型存取權的首選方式。
              </p>
              
              <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap", alignItems: "center" }}>
                {availableKeys.length > 0 && (
                  <select 
                    className="gb-input" 
                    style={{ flex: "1 1 200px", maxWidth: "250px", cursor: "pointer", fontFamily: "monospace", minWidth: "200px" }}
                    value={availableKeys.includes(nativeKey) ? nativeKey : ""}
                    onChange={e => setNativeKey(e.target.value)}
                  >
                    <option value="" disabled>-- 快速選擇庫存金鑰 --</option>
                    {availableKeys.map((k, i) => {
                      const masked = k.length > 15 ? `${k.substring(0, 8)}...${k.substring(k.length - 4)}` : k;
                      return <option key={i} value={k}>Key {i+1}: {masked}</option>;
                    })}
                  </select>
                )}
                <input 
                  type="password" 
                  className="gb-input" 
                  placeholder={availableKeys.length > 0 ? "或在此手動輸入/貼上 API Key" : "請貼上欲測試的 API Key (通常為 AIzaSy...)"}
                  value={nativeKey} 
                  onChange={e => setNativeKey(e.target.value)} 
                  style={{ flex: "2 1 300px", fontFamily: "monospace" }}
                />
                <button 
                  className="btn" 
                  onClick={testNative} 
                  disabled={nativeLoading} 
                  style={{ 
                    flex: "0 0 auto", background: "linear-gradient(135deg, #7c3aed, #c084fc)", 
                    color: "#fff", border: "none", padding: "0.6rem 1.5rem", boxShadow: "0 4px 6px -1px rgba(124, 58, 237, 0.4)" 
                  }}
                >
                  {nativeLoading ? <><i className="fas fa-spinner fa-spin" /> 聯網驗證中...</> : <><i className="fas fa-radar" /> 執行診斷</>}
                </button>
              </div>

              {nativeError && (
                <div style={{ marginTop: "1.5rem", color: "#fca5a5", background: "#7f1d1d", padding: "1rem", borderRadius: "8px", border: "1px solid #b91c1c" }}>
                  <i className="fas fa-triangle-exclamation" style={{ marginRight: "0.5rem" }} /> {nativeError}
                </div>
              )}
            </div>
            
            {/* Native Models Display */}
            {nativeModels && (
              <div className="fade-in-up">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1.5rem", paddingBottom: "1rem", borderBottom: "1px solid #334155" }}>
                  <h3 style={{ color: "#4ade80", margin: 0 }}>
                    <i className="fas fa-check-shield" /> 鑑權通過！共取得 {nativeModels.length} 個原生模型存取權。
                  </h3>
                  <a href="https://aistudio.google.com/app/plan_information" target="_blank" rel="noreferrer" style={{ color: "#60a5fa", fontSize: "0.85rem", textDecoration: "none", display: "flex", alignItems: "center", gap: "0.3rem", background: "#1e3a8a", padding: "0.4rem 0.8rem", borderRadius: "20px" }}>
                    <i className="fas fa-chart-line" /> 查看 Rate Limit & Token Usage
                  </a>
                </div>
                
                {Object.entries(groupedNativeModels)
                  .sort(([a], [b]) => a.localeCompare(b))
                  .map(([family, models]) => (
                    <div key={family} style={{ marginBottom: "2.5rem" }}>
                      <h4 style={{ color: "#94a3b8", fontSize: "1.1rem", marginBottom: "1rem", textTransform: "uppercase", letterSpacing: "1px", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                        <i className={`fas ${family.includes("Gemini") ? "fa-sparkles" : family.includes("Gemma") ? "fa-brain" : family.includes("Imagen") ? "fa-image" : "fa-cube"}`} />
                        {family} <span style={{ background: "#334155", color: "#cbd5e1", padding: "0.1rem 0.5rem", borderRadius: "10px", fontSize: "0.75rem" }}>{models.length}</span>
                      </h4>
                      
                      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))", gap: "1rem" }}>
                        {models.map(m => (
                          <div key={m.id} style={{ 
                            background: "linear-gradient(145deg, #1e293b, #0f172a)", 
                            border: "1px solid #334155", 
                            borderRadius: "12px", 
                            padding: "1rem",
                            transition: "all 0.2s",
                            boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
                            cursor: "default"
                          }}
                          onMouseEnter={e => e.currentTarget.style.borderColor = "#c084fc"}
                          onMouseLeave={e => e.currentTarget.style.borderColor = "#334155"}
                          >
                            <div style={{ color: "#f8fafc", fontWeight: "600", fontSize: "1rem", marginBottom: "0.3rem", wordBreak: "break-all" }}>
                              {m.displayName}
                            </div>
                            <div style={{ color: "#818cf8", fontSize: "0.8rem", fontFamily: "monospace", wordBreak: "break-all" }}>
                              {m.id}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ----------------- PROXY 測試 TAB ----------------- */}
        {activeTab === "proxy" && (
          <div className="fade-in">
            <div style={{ background: "#1e293b", padding: "1.5rem", borderRadius: "10px", border: "1px solid #334155", marginBottom: "2rem" }}>
              <h4 style={{ marginBottom: "1rem", color: "#38bdf8", display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <i className="fas fa-network-wired" /> 系統對外映射配置健檢
              </h4>
              <p style={{ color: "#94a3b8", fontSize: "0.95rem", lineHeight: "1.5", margin: 0 }}>
                驗證您的 Gemini Pool Proxy 是否準備好接受客戶端的相容 OpenAI 格式請求。此操作會模擬外部應用程式，取用系統後台令牌發送請求給 `127.0.0.1:18080/v1/models` 端點。
              </p>
              
              <div style={{ marginTop: "1.5rem", display: "flex" }}>
                <button 
                  className="btn" 
                  onClick={testProxy} 
                  disabled={proxyLoading} 
                  style={{ 
                    background: "linear-gradient(135deg, #0284c7, #38bdf8)", 
                    color: "#fff", border: "none", padding: "0.6rem 1.5rem", boxShadow: "0 4px 6px -1px rgba(2, 132, 199, 0.4)" 
                  }}
                >
                  {proxyLoading ? <><i className="fas fa-spinner fa-spin" /> 連線映射端點中...</> : <><i className="fas fa-bolt" /> 觸發本機端點測試</>}
                </button>
              </div>

              {proxyError && (
                <div style={{ marginTop: "1.5rem", color: "#fca5a5", background: "#7f1d1d", padding: "1rem", borderRadius: "8px", border: "1px solid #b91c1c" }}>
                  <i className="fas fa-triangle-exclamation" style={{ marginRight: "0.5rem" }} /> {proxyError}
                </div>
              )}
            </div>

            {proxyModels && (
              <div className="fade-in-up">
                <div style={{ marginBottom: "1.5rem", paddingBottom: "1rem", borderBottom: "1px solid #334155" }}>
                  <h3 style={{ color: "#4ade80", margin: 0 }}>
                    <i className="fas fa-check-shield" /> Proxy 映射正常！當前開放了 {proxyModels.length} 個相容模型。
                  </h3>
                </div>
                
                <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: "1rem" }}>
                  {proxyModels.map(m => (
                    <div key={m.id} style={{ 
                      background: "linear-gradient(145deg, #1e293b, #0f172a)", 
                      border: "1px solid #334155", 
                      borderRadius: "12px", 
                      padding: "1.2rem 1rem",
                      display: "flex", alignItems: "center", gap: "1rem"
                    }}>
                      <div style={{ width: "40px", height: "40px", borderRadius: "8px", background: "#0ea5e9", color: "#fff", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "1.2rem", flexShrink: 0 }}>
                        <i className={m.id.includes("vision") || m.id.includes("image") ? "fas fa-image" : m.id.includes("audio") ? "fas fa-microphone" : "fas fa-cube"} />
                      </div>
                      <div>
                        <div style={{ color: "#f8fafc", fontWeight: "600", fontSize: "1rem", wordBreak: "break-all" }}>
                          {m.id}
                        </div>
                        <div style={{ color: "#cbd5e1", fontSize: "0.8rem", marginTop: "0.2rem" }}>
                          相容格式開放
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
        
      </div>
      
      {/* 簡單的 inline 動畫宣告 */}
      <style>{`
        .fade-in { animation: fadeIn 0.3s ease-in-out; }
        .fade-in-up { animation: fadeInUp 0.4s cubic-bezier(0.16, 1, 0.3, 1); }
        @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
        @keyframes fadeInUp { from { opacity: 0; transform: translateY(15px); } to { opacity: 1; transform: translateY(0); } }
      `}</style>
    </div>
  );
}
