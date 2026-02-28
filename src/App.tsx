import { useEffect, useState } from "react";
import { CompanionWindow } from "./components/companion/CompanionWindow";
import { OverlayWindow } from "./components/overlay/OverlayWindow";

function App() {
  const [windowLabel, setWindowLabel] = useState<string>("companion");

  useEffect(() => {
    async function getLabel() {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const win = getCurrentWindow();
        setWindowLabel(win.label);
      } catch {
        // Running outside Tauri (e.g. vite dev)
        setWindowLabel("companion");
      }
    }
    getLabel();
  }, []);

  if (windowLabel === "overlay") {
    return <OverlayWindow />;
  }

  return <CompanionWindow />;
}

export default App;
