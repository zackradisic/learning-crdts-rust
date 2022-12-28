import { useAppState } from "./state";

const Cursors = () => {
  const cursors = useAppState((state) => state.cursors);

  return (
    <>
      {Object.entries(cursors).map(([id, [x, y]]) => (
        <Cursor id={id + ""} x={x} y={y} key={id} />
      ))}
    </>
  );
};

type Props = {
  id: string;
  x: number;
  y: number;
};

const Cursor = ({ id, x, y }: Props) => {
  return (
    <svg
      x={x}
      y={y}
      //   style={{ transform: `translate(${x}px, ${y}px)` }}
      className="cursor "
      stroke="red"
      width="16"
      height="20"
      viewBox="0 0 16 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M2.06266 2.34964L1.12849 1.65076L1.26666 2.80921L2.92019 16.6732L3.08745 18.0757L3.83976 16.8804L3.83977 16.8804L3.83982 16.8803L3.83988 16.8803L3.84063 16.8791L3.84422 16.8734L3.85956 16.8494C3.87335 16.8279 3.8942 16.7957 3.92163 16.754C3.97651 16.6705 4.05766 16.549 4.16136 16.3991C4.36899 16.0992 4.66584 15.6873 5.02218 15.2405C5.74562 14.3333 6.67404 13.3339 7.57284 12.8014C8.49365 12.2559 9.8179 11.9519 10.9582 11.7891C11.5198 11.7089 12.0207 11.665 12.3809 11.6412C12.5608 11.6293 12.705 11.6224 12.8036 11.6186C12.8528 11.6167 12.8906 11.6155 12.9157 11.6148L12.9436 11.6141L12.9501 11.6139L12.9514 11.6139L12.9514 11.6139L12.9515 11.6139L12.9516 11.6139L14.4127 11.5891L13.2425 10.7136L2.06266 2.34964Z"
        fill="black"
        stroke="white"
      />
    </svg>
  );
};

export default Cursors;
