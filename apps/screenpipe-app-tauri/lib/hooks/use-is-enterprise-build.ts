"use client";

import { useState, useEffect } from "react";
import { commands } from "@/lib/utils/tauri";

/** True when running the enterprise build (updates managed by IT). */
export function useIsEnterpriseBuild(): boolean {
  const [isEnterprise, setIsEnterprise] = useState(false);

  useEffect(() => {
    commands
      .isEnterpriseBuildCmd()
      .then(setIsEnterprise)
      .catch(() => setIsEnterprise(false));
  }, []);

  return isEnterprise;
}
