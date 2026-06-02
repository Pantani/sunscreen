// Sunscreen — Mermaid theme aligned with Eclipse palette.
(function () {
  if (typeof window === "undefined") return;
  window.addEventListener("load", function () {
    if (typeof mermaid === "undefined") return;
    mermaid.initialize({
      startOnLoad: true,
      theme: "base",
      themeVariables: {
        background: "#0B0D12",
        primaryColor: "#13161D",
        primaryTextColor: "#E8E6E1",
        primaryBorderColor: "#2A2F3A",
        lineColor: "#8A8B92",
        secondaryColor: "#1A1E27",
        tertiaryColor: "#13161D",
        nodeBorder: "#F4A340",
        edgeLabelBackground: "#0B0D12",
        fontFamily: "Inter, -apple-system, sans-serif",
        fontSize: "14px"
      },
      flowchart: { curve: "basis", padding: 14 },
      sequence: { actorMargin: 50, mirrorActors: false }
    });
  });
})();
