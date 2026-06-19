import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface ProgressPayload {
  stage: string;
  percent: number;
}

export function useProgress() {
  const [stage, setStage] = useState<string>("");
  const [percent, setPercent] = useState<number>(0);
  const [isActive, setIsActive] = useState(false);

  useEffect(() => {
    const unlisten = listen<ProgressPayload>("progress", (event) => {
      setStage(event.payload.stage);
      setPercent(event.payload.percent);
      setIsActive(event.payload.percent < 100);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const reset = () => {
    setStage("");
    setPercent(0);
    setIsActive(false);
  };

  return { stage, percent, isActive, reset };
}
