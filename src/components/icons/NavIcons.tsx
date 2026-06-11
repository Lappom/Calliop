import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const iconDefaults: IconProps = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
};

export function IconHome(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <path d="M2.5 6.5 8 2l5.5 4.5V13a1 1 0 0 1-1 1H3.5a1 1 0 0 1-1-1V6.5Z" />
      <path d="M6 14V9h4v5" />
    </svg>
  );
}

export function IconDictionary(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <path d="M3 2.5h4a2 2 0 0 1 2 2V13a1.5 1.5 0 0 0-1.5-1.5H3V2.5Z" />
      <path d="M13 2.5H9a2 2 0 0 0-2 2V13a1.5 1.5 0 0 1 1.5-1.5H13V2.5Z" />
    </svg>
  );
}

export function IconSnippets(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <path d="M5.5 4.5h7M5.5 8h5M5.5 11.5h7" />
      <rect x="2.5" y="2.5" width="11" height="11" rx="1.5" />
    </svg>
  );
}

export function IconContext(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <rect x="2" y="3" width="12" height="10" rx="1.5" />
      <path d="M5 6.5h6M5 9h4" />
      <circle cx="12" cy="4" r="1.5" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function IconInsight(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <path d="M2.5 12.5V9l2-4.5h7l2 4.5v3.5" />
      <path d="M2.5 12.5h11" />
      <path d="M6 9.5h4" />
    </svg>
  );
}

export function IconSettings(props: IconProps) {
  return (
    <svg {...iconDefaults} {...props}>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1" />
    </svg>
  );
}
