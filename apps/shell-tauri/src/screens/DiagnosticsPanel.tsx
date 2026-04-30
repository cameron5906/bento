import React from "react";

interface Props {
  technicalMessage: string;
}

export function DiagnosticsPanel({ technicalMessage }: Props) {
  return (
    <div style={styles.panel}>
      <h4 style={styles.heading}>Technical Details</h4>
      <pre style={styles.pre}>{technicalMessage}</pre>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  panel: {
    marginTop: 20,
    padding: 16,
    background: "#1a1a1a",
    borderRadius: 8,
    border: "1px solid #333",
    textAlign: "left",
  },
  heading: {
    fontSize: 12,
    fontWeight: 600,
    color: "#888",
    textTransform: "uppercase" as const,
    letterSpacing: 1,
    marginBottom: 8,
  },
  pre: {
    fontSize: 12,
    color: "#ccc",
    fontFamily: "'Cascadia Code', 'Fira Code', monospace",
    whiteSpace: "pre-wrap" as const,
    wordBreak: "break-all" as const,
    margin: 0,
  },
};
