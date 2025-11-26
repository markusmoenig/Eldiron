macro_rules! const_array {
    (
        $vis:vis $name:ident: $ty:ty [
            $($item:expr),*
        ]
    ) => {
        $vis const $name: [$ty; count_tts!($($item)*)] = [
            $(
                $item,
            )*
        ];
    };
}

macro_rules! count_tts {
    ($($tts:tt)*) => {0usize $(+ replace_expr!($tts 1usize))*};
}

macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}
