use glib::ObjectExt;
use gtk4::gdk_pixbuf::prelude::PixbufLoaderExt;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::{gdk_pixbuf, glib, subclass::prelude::*};

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::gdk_pixbuf::prelude::StaticType;
    use gtk4::gdk_pixbuf::Pixbuf;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug, Default)]
    pub struct RowData {
        id: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        pub(crate) asset: RefCell<Option<egs_api::api::types::asset_info::AssetInfo>>,
        thumbnail: RefCell<Option<Pixbuf>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for RowData {
        const NAME: &'static str = "RowData";
        type Type = super::RowData;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                id: RefCell::new(None),
                name: RefCell::new(None),
                asset: RefCell::new(None),
                thumbnail: RefCell::new(None),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for RowData {
        fn properties() -> &'static [glib::ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_string(
                        "name",
                        "Name",
                        "Name",
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
                "name" => {
                    let name = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.name.replace(name);
                }
                "id" => {
                    let id = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.id.replace(id);
                }
                "thumbnail" => {
                    let thumbnail = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.thumbnail.replace(thumbnail);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "id" => self.id.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the RowData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct RowData(ObjectSubclass<imp::RowData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl RowData {
    pub fn new(asset: &egs_api::api::types::asset_info::AssetInfo, image: &[u8]) -> RowData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create RowData");
        let self_: &imp::RowData = imp::RowData::from_instance(&data);

        data.set_property("id", &asset.id).unwrap();
        data.set_property("name", &asset.title).unwrap();
        self_.asset.replace(Some(asset.clone()));

        let pixbuf_loader = gdk_pixbuf::PixbufLoader::new();
        pixbuf_loader.write(image).unwrap();
        pixbuf_loader.close().ok();

        if let Some(pix) = pixbuf_loader.pixbuf() {
            data.set_property("thumbnail", &pix).unwrap();
        };
        data
    }

    pub fn id(&self) -> String {
        if let Ok(value) = self.property("id") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn name(&self) -> String {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return id_opt;
            }
        };
        "".to_string()
    }

    pub fn image(&self) -> Option<Pixbuf> {
        if let Ok(value) = self.property("thumbnail") {
            if let Ok(id_opt) = value.get::<Pixbuf>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn check_category(&self, cat: String) -> bool {
        let self_: &imp::RowData = imp::RowData::from_instance(self);
        match self_.asset.borrow().as_ref() {
            None => false,
            Some(b) => {
                for category in b.categories.as_ref().unwrap() {
                    for split in cat.split('|') {
                        if category
                            .path
                            .to_ascii_lowercase()
                            .contains(&split.to_ascii_lowercase())
                        {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}