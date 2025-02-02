use gtk4::glib::clone;
use gtk4::subclass::prelude::*;
use gtk4::{self, prelude::*};
use gtk4::{glib, CompositeTemplate};

pub(crate) mod imp {
    use super::*;
    use gtk4::gdk_pixbuf::prelude::StaticType;
    use gtk4::gdk_pixbuf::Pixbuf;
    use gtk4::glib::SignalHandlerId;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/achetagames/epic_asset_manager/asset.ui")]
    pub struct EpicAsset {
        id: RefCell<Option<String>>,
        label: RefCell<Option<String>>,
        favorite: RefCell<bool>,
        downloaded: RefCell<bool>,
        thumbnail: RefCell<Option<Pixbuf>>,
        #[template_child]
        pub image: TemplateChild<gtk4::Image>,
        pub data: RefCell<Option<crate::models::asset_data::AssetData>>,
        pub handler: RefCell<Option<SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpicAsset {
        const NAME: &'static str = "EpicAsset";
        type Type = super::EpicAsset;
        type ParentType = gtk4::Box;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                label: RefCell::new(None),
                favorite: RefCell::new(false),
                downloaded: RefCell::new(false),
                thumbnail: RefCell::new(None),
                image: TemplateChild::default(),
                data: RefCell::new(None),
                handler: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpicAsset {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "label",
                        "Label",
                        "Label",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_string(
                        "id",
                        "ID",
                        "ID",
                        None, // Default value
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_object(
                        "thumbnail",
                        "Thumbnail",
                        "Thumbnail",
                        Pixbuf::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boolean(
                        "favorite",
                        "favorite",
                        "Is favorite",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_boolean(
                        "downloaded",
                        "downloaded",
                        "Is Downloaded",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "label" => {
                    let label = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.label.replace(label);
                }
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "favorite" => {
                    let favorite = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.favorite.replace(favorite);
                }
                "downloaded" => {
                    let downloaded = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.downloaded.replace(downloaded);
                }
                "thumbnail" => {
                    let thumbnail: Option<Pixbuf> = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");

                    self.thumbnail.replace(thumbnail.clone());
                    match thumbnail {
                        None => {
                            self.image.set_icon_name(Some("ue-logo-symbolic"));
                        }
                        Some(t) => {
                            self.image.set_from_pixbuf(Some(t.as_ref()));
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "label" => self.label.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "favorite" => self.favorite.borrow().to_value(),
                "downloaded" => self.downloaded.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for EpicAsset {}
    impl BoxImpl for EpicAsset {}
}

glib::wrapper! {
    pub struct EpicAsset(ObjectSubclass<imp::EpicAsset>)
        @extends gtk4::Widget, gtk4::Box;
}

impl Default for EpicAsset {
    fn default() -> Self {
        Self::new()
    }
}

impl EpicAsset {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create EpicLibraryBox")
    }

    pub fn set_data(&self, data: &crate::models::asset_data::AssetData) {
        let self_: &imp::EpicAsset = imp::EpicAsset::from_instance(self);
        if let Some(d) = self_.data.take() {
            if let Some(id) = self_.handler.take() {
                d.disconnect(id);
            }
        }
        self_.data.replace(Some(data.clone()));
        self.set_property("label", &data.name()).unwrap();
        self.set_property("thumbnail", &data.image()).unwrap();
        self.set_property("favorite", &data.favorite()).unwrap();
        self.set_property("downloaded", &data.downloaded()).unwrap();
        if let Ok(id) = data.connect_local(
            "refreshed",
            false,
            clone!(@weak self as asset, @weak data => @default-return None, move |_| {
                asset.set_property("favorite", &data.favorite()).unwrap();
                None
            }),
        ) {
            self_.handler.replace(Some(id));
        }
    }
}
