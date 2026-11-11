/** Opens the Add card dialog from anywhere; `added` counts cards added, so lists can refresh. */
export const addCardDialog: { open: boolean; added: number } = $state({ open: false, added: 0 });
