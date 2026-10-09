use leptos::prelude::*;

#[component]
pub fn ConfirmModal(
    title: &'static str,
    confirm_label: &'static str,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class="modal d-block"
            id="confirm-modal"
            tabindex="-1"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
        >
            <div class="modal-dialog modal-dialog-centered">
                <div class="modal-content">
                    <div class="modal-header">
                        <h5 class="modal-title" id="confirm-title">
                            {title}
                        </h5>
                        <button
                            type="button"
                            class="btn-close"
                            aria-label="Close"
                            on:click=move |_| on_cancel.run(())
                        ></button>
                    </div>
                    <div class="modal-body">{children()}</div>
                    <div class="modal-footer">
                        <button
                            type="button"
                            class="btn btn-secondary"
                            id="confirm-cancel"
                            on:click=move |_| on_cancel.run(())
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="btn btn-danger"
                            id="confirm-accept"
                            on:click=move |_| on_confirm.run(())
                        >
                            {confirm_label}
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <div class="modal-backdrop show"></div>
    }
}
