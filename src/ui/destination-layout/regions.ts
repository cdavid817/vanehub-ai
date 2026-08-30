import type { ReactNode } from "react";

export interface HorizontalPaneRegion {
  content: ReactNode;
  open: boolean;
  width: number;
  min: number;
  max: number;
  onWidthChange: (width: number) => void;
  onWidthCommit?: (width: number) => void;
  onOpenChange: (open: boolean) => void;
  /** Accessible label for both the resize gutter and the sheet title at narrower tiers. */
  label: string;
}

export interface RuntimePanelRegion {
  content: ReactNode;
  open: boolean;
  height: number;
  min: number;
  max: number;
  onHeightChange: (height: number) => void;
  onHeightCommit?: (height: number) => void;
  label: string;
}
