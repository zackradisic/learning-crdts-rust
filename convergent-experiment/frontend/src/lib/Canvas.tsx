import { RefObject, useRef } from "react";
import { useDrag } from "@use-gesture/react";
import { Square, SquareId } from "./proto/types";
import { useAppState } from "./state";
import * as Interactions from "./interactions";
import Cursors from "./Cursor";

const clamp = (num: number, min: number, max: number) =>
  Math.max(Math.min(num, max), min);

const Canvas = () => {
  const squares = useAppState((state) => state.squares);
  const ref = useRef<SVGSVGElement>(null);
  return (
    <svg
      onMouseMove={(e) => {
        const svg = ref.current;
        if (!svg) return;

        const point = svg.createSVGPoint();

        point.x = e.clientX;
        point.y = e.clientY;

        const cursorPoint = point.matrixTransform(
          svg.getScreenCTM()?.inverse()
        );

        Interactions.updateCursor(cursorPoint.x, cursorPoint.y);

        // Interactions.updateCursor(e.clientX, e.clientY);
      }}
      className="h-full w-full rounded bg-[#1e1e1e]"
      viewBox="0 0 1000 1000"
      ref={ref}
      onClick={(e) => {
        // Point transformation method from here:
        // https://stackoverflow.com/questions/29261304/how-to-get-the-click-coordinates-relative-to-svg-element-holding-the-onclick-lis

        const svg = ref.current;
        if (!svg) return;

        const point = svg.createSVGPoint();

        point.x = e.clientX;
        point.y = e.clientY;

        const cursorPoint = point.matrixTransform(
          svg.getScreenCTM()?.inverse()
        );
        const id = Math.floor(Math.random() * 10000);
        Interactions.setSquare(id, {
          x: clamp(cursorPoint.x, 0, 1000 - 100),
          y: clamp(cursorPoint.y, 0, 1000 - 100),
          width: 100,
          height: 100,
        });
      }}
    >
      {Object.entries(squares).map(([id, square]) => (
        <Square key={id} svg={ref} id={id} square={square} />
      ))}
      <Cursors />
    </svg>
  );
};

type Props = {
  id: SquareId;
  square: Square;
  svg: RefObject<SVGSVGElement>;
};

const Square = ({ id, square, svg: svgRef }: Props) => {
  const cachedPoint = useRef<DOMPoint | undefined>(undefined);
  const bind = useDrag(({ down, xy: [mx, my], event }) => {
    const svg = svgRef.current;
    if (svg === null) return;
    // event.stopPropagation();
    // event.preventDefault();
    let point = cachedPoint.current;
    if (!point) {
      point = svg.createSVGPoint();
      cachedPoint.current = point;
    }
    point.x = mx;
    point.y = my;

    const cursorPoint = point.matrixTransform(svg.getScreenCTM()?.inverse());

    Interactions.setSquare(parseInt(id), {
      ...square,
      x: cursorPoint.x,
      y: cursorPoint.y,
    });

    // Interactions.setSquare(parseInt(id), {
    //   ...square,
    //   x: square.x + mx,
    //   y: square.y + my,
    // });
  });
  return (
    <>
      <rect
        {...bind()}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          Interactions.removeSquare(parseInt(id));
        }}
        className="rounded-lg bg-red-400"
        rx={3}
        x={square.x}
        y={square.y}
        stroke="#91c9f9"
        fill="#91c9f9"
        width={square.width}
        height={square.height}
      ></rect>
      <text
        className="pointer-events-none"
        fill="black"
        x={square.x + 5}
        y={square.y + 20}
        fontSize={15}
        width={square.width / 2}
        height={square.height / 2}
      >
        {id}
      </text>
      <text
        className="pointer-events-none"
        fill="black"
        x={square.x + 5}
        y={square.y + square.height - 10}
        fontSize={15}
        width={square.width / 2}
        height={square.height / 2}
      >
        {/* {square.x} {square.y} */}
      </text>
    </>
  );
};

export default Canvas;
