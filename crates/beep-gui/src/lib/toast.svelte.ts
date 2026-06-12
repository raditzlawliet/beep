let _title = $state("");
let _description = $state("");
let _id = $state(0);

export function showToast(title: string, description: string) {
  _title = title;
  _description = description;
  _id += 1;
}

export function closeToast() {
  _title = "";
  _description = "";
}

export function getToastState() {
  return {
    get title() {
      return _title;
    },
    get description() {
      return _description;
    },
    get id() {
      return _id;
    },
  };
}
