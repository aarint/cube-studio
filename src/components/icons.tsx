/**
 * Icon set.
 *
 * Hand-drawn 16px glyphs on a 16×16 grid with a 1.5px stroke. Keeping them local
 * avoids an icon-font dependency in a desktop bundle and lets every glyph share
 * one optical weight.
 */

import type { JSX, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function Svg({ size = 16, children, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {children}
    </svg>
  );
}

export const IconDatabase = (props: IconProps) => (
  <Svg {...props}>
    <ellipse cx="8" cy="4" rx="5" ry="2" />
    <path d="M3 4v4c0 1.1 2.2 2 5 2s5-.9 5-2V4" />
    <path d="M3 8v4c0 1.1 2.2 2 5 2s5-.9 5-2V8" />
  </Svg>
);

export const IconLayers = (props: IconProps) => (
  <Svg {...props}>
    <path d="M8 2 2 5l6 3 6-3-6-3Z" />
    <path d="M2 8l6 3 6-3" />
    <path d="M2 11l6 3 6-3" />
  </Svg>
);

export const IconQueue = (props: IconProps) => (
  <Svg {...props}>
    <rect x="2" y="3" width="12" height="2.5" rx="1" />
    <rect x="2" y="7" width="12" height="2.5" rx="1" />
    <rect x="2" y="11" width="7" height="2.5" rx="1" />
  </Svg>
);

export const IconRadio = (props: IconProps) => (
  <Svg {...props}>
    <circle cx="8" cy="8" r="1.75" />
    <path d="M4.5 4.5a5 5 0 0 0 0 7M11.5 4.5a5 5 0 0 1 0 7" />
    <path d="M2 2a8.5 8.5 0 0 0 0 12M14 2a8.5 8.5 0 0 1 0 12" opacity=".5" />
  </Svg>
);

export const IconTopic = (props: IconProps) => (
  <Svg {...props}>
    <path d="M2 5h4M10 5h4M2 11h4M10 11h4" />
    <circle cx="8" cy="5" r="1.75" />
    <circle cx="8" cy="11" r="1.75" />
  </Svg>
);

export const IconChat = (props: IconProps) => (
  <Svg {...props}>
    <path d="M3 3h10a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H6l-3 2.5V4a1 1 0 0 1 1-1Z" />
  </Svg>
);

export const IconSearch = (props: IconProps) => (
  <Svg {...props}>
    <circle cx="7" cy="7" r="4" />
    <path d="m10.5 10.5 3 3" />
  </Svg>
);

export const IconPlus = (props: IconProps) => (
  <Svg {...props}>
    <path d="M8 3v10M3 8h10" />
  </Svg>
);

export const IconClose = (props: IconProps) => (
  <Svg {...props}>
    <path d="m4 4 8 8M12 4l-8 8" />
  </Svg>
);

export const IconEdit = (props: IconProps) => (
  <Svg {...props}>
    <path d="M11 2.5 13.5 5 6 12.5 3 13l.5-3L11 2.5Z" />
  </Svg>
);

export const IconTrash = (props: IconProps) => (
  <Svg {...props}>
    <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.6 9.5h6.8L12 4" />
  </Svg>
);

export const IconRefresh = (props: IconProps) => (
  <Svg {...props}>
    <path d="M13.5 8a5.5 5.5 0 1 1-1.7-4" />
    <path d="M13.5 2v3h-3" />
  </Svg>
);

export const IconTerminal = (props: IconProps) => (
  <Svg {...props}>
    <rect x="2" y="3" width="12" height="10" rx="1.5" />
    <path d="m5 7 2 2-2 2M8.5 11h2.5" />
  </Svg>
);

export const IconCheck = (props: IconProps) => (
  <Svg {...props}>
    <path d="m3 8.5 3.5 3.5L13 5" />
  </Svg>
);

export const IconAlert = (props: IconProps) => (
  <Svg {...props}>
    <path d="M8 2.5 14.5 13.5h-13L8 2.5Z" />
    <path d="M8 6.5v3M8 11.5v.01" />
  </Svg>
);

export const IconKey = (props: IconProps) => (
  <Svg {...props}>
    <circle cx="5.5" cy="10.5" r="2.5" />
    <path d="m7.5 8.5 5-5M10.5 5.5l1.5 1.5M11.5 4.5 13 6" />
  </Svg>
);

export const IconChevronRight = (props: IconProps) => (
  <Svg {...props}>
    <path d="m6 3.5 4.5 4.5L6 12.5" />
  </Svg>
);

export const IconCopy = (props: IconProps) => (
  <Svg {...props}>
    <rect x="5.5" y="5.5" width="8" height="8" rx="1" />
    <path d="M10.5 5.5V3a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1v6.5a1 1 0 0 0 1 1h2.5" />
  </Svg>
);

export const IconClock = (props: IconProps) => (
  <Svg {...props}>
    <circle cx="8" cy="8" r="6" />
    <path d="M8 4.5V8l2.5 1.5" />
  </Svg>
);

export const IconTls = (props: IconProps) => (
  <Svg {...props}>
    <rect x="3" y="7" width="10" height="7" rx="1.5" />
    <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
  </Svg>
);

/** Glyph for a backend kind, used anywhere a connection is listed. */
export const BACKEND_ICONS: Record<string, (props: IconProps) => JSX.Element> = {
  redis: IconDatabase,
  memcached: IconLayers,
  kafka: IconQueue,
  rabbitmq: IconChat,
  nats: IconRadio,
  mqtt: IconTopic,
};

/** Glyph for a container kind. */
export const CONTAINER_ICONS: Record<string, (props: IconProps) => JSX.Element> = {
  redis_db: IconDatabase,
  memcached_server: IconLayers,
  kafka_topic: IconQueue,
  rabbit_queue: IconChat,
  rabbit_exchange: IconChevronRight,
  nats_stream: IconRadio,
  nats_subject: IconRadio,
  mqtt_topic: IconTopic,
};

export function BackendGlyph({
  kind,
  size = 16,
  ...rest
}: { kind: string } & IconProps) {
  const Glyph = BACKEND_ICONS[kind] ?? IconDatabase;
  return <Glyph size={size} {...rest} />;
}

export function ContainerGlyph({
  kind,
  size = 14,
  ...rest
}: { kind: string } & IconProps) {
  const Glyph = CONTAINER_ICONS[kind] ?? IconKey;
  return <Glyph size={size} {...rest} />;
}
