import React from "react";
import { Button } from "@openai/apps-sdk-ui/components/Button";
import { Image } from "@openai/apps-sdk-ui/components/Image";

function AlbumCard({ album, onSelect }) {
  return (
    <Button
      type="button"
      variant="ghost"
      color="secondary"
      pill={false}
      className="group relative flex-shrink-0 w-[272px] bg-white text-left p-0 h-auto min-h-0 rounded-none shadow-none gap-0 before:hidden"
      onClick={() => onSelect?.(album)}
    >
      <div className="flex w-full flex-col gap-2">
        <div className="relative aspect-[4/3] w-full overflow-hidden rounded-2xl shadow-lg">
          <Image
            src={album.cover}
            alt={album.title}
            className="h-full w-full object-cover"
            loading="lazy"
          />
        </div>
        <div className="px-1.5">
          <div className="text-base font-medium truncate">{album.title}</div>
          <div className="mt-0.5 text-sm font-normal text-black/60">
            {album.photos.length} photos
          </div>
        </div>
      </div>
    </Button>
  );
}

export default AlbumCard;
