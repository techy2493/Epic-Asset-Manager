use glib::ObjectExt;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::glib::clone;
use gtk4::{self, glib, prelude::*, subclass::prelude::*};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::thread;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Uproject {
    #[serde(default)]
    pub file_version: i64,
    #[serde(default)]
    pub engine_association: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    pub modules: Option<Vec<Module>>,
    pub plugins: Option<Vec<Plugin>>,
    pub disable_engine_plugins_by_default: Option<bool>,
    pub enterprise: Option<bool>,
    pub additional_plugin_directories: Option<Vec<String>>,
    pub additional_root_directories: Option<Vec<String>>,
    pub target_platforms: Option<Vec<String>>,
    pub epic_sample_name_hash: Option<String>,
    pub pre_build_steps: Option<HashMap<String, Vec<String>>>,
    pub post_build_steps: Option<HashMap<String, Vec<String>>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Module {
    pub name: String,
    #[serde(rename = "Type", default)]
    pub type_field: String,
    #[serde(default)]
    pub loading_phase: String,
    pub additional_dependencies: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Plugin {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub marketplace_url: Option<String>,
    #[serde(default)]
    pub supported_target_platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum ProjectMsg {
    Thumbnail(Vec<u8>),
}

// Implementation sub-module of the GObject
mod imp {
    use super::*;
    use glib::ToValue;
    use gtk4::gdk_pixbuf::prelude::StaticType;
    use gtk4::gdk_pixbuf::Pixbuf;
    use gtk4::glib::ParamSpec;
    use std::cell::RefCell;

    // The actual data structure that stores our values. This is not accessible
    // directly from the outside.
    #[derive(Debug)]
    pub struct ProjectData {
        guid: RefCell<Option<String>>,
        path: RefCell<Option<String>>,
        name: RefCell<Option<String>>,
        pub uproject: RefCell<Option<super::Uproject>>,
        thumbnail: RefCell<Option<Pixbuf>>,
        pub sender: gtk4::glib::Sender<super::ProjectMsg>,
        pub receiver: RefCell<Option<gtk4::glib::Receiver<super::ProjectMsg>>>,
    }

    // Basic declaration of our type for the GObject type system
    #[glib::object_subclass]
    impl ObjectSubclass for ProjectData {
        const NAME: &'static str = "ProjectData";
        type Type = super::ProjectData;
        type ParentType = glib::Object;

        fn new() -> Self {
            let (sender, receiver) = gtk4::glib::MainContext::channel(gtk4::glib::PRIORITY_DEFAULT);
            Self {
                guid: RefCell::new(None),
                path: RefCell::new(None),
                name: RefCell::new(None),
                uproject: RefCell::new(None),
                thumbnail: RefCell::new(None),
                sender,
                receiver: RefCell::new(Some(receiver)),
            }
        }
    }

    // The ObjectImpl trait provides the setters/getters for GObject properties.
    // Here we need to provide the values that are internally stored back to the
    // caller, or store whatever new value the caller is providing.
    //
    // This maps between the GObject properties and our internal storage of the
    // corresponding values of the properties.
    impl ObjectImpl for ProjectData {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_messaging();
        }

        fn signals() -> &'static [gtk4::glib::subclass::Signal] {
            static SIGNALS: once_cell::sync::Lazy<Vec<gtk4::glib::subclass::Signal>> =
                once_cell::sync::Lazy::new(|| {
                    vec![gtk4::glib::subclass::Signal::builder(
                        "finished",
                        &[],
                        <()>::static_type().into(),
                    )
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
                });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "guid",
                        "GUID",
                        "GUID",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "path",
                        "Path",
                        "Path",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "name",
                        "Name",
                        "Name",
                        None,
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
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "guid" => {
                    let guid = value.get().unwrap();
                    self.guid.replace(guid);
                }
                "path" => {
                    let path = value.get().unwrap();
                    self.path.replace(path);
                }
                "name" => {
                    let name = value.get().unwrap();
                    self.name.replace(name);
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

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "guid" => self.guid.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "name" => self.name.borrow().to_value(),
                "thumbnail" => self.thumbnail.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

// Public part of the ProjectData type. This behaves like a normal gtk-rs-style GObject
// binding
glib::wrapper! {
    pub struct ProjectData(ObjectSubclass<imp::ProjectData>);
}

// Constructor for new instances. This simply calls glib::Object::new() with
// initial values for our two properties and then returns the new instance
impl ProjectData {
    pub fn new(path: &str, name: &str) -> ProjectData {
        let data: Self = glib::Object::new(&[]).expect("Failed to create ProjectData");
        let self_: &imp::ProjectData = imp::ProjectData::from_instance(&data);
        data.set_property("path", &path).unwrap();
        data.set_property("name", &name).unwrap();
        let mut uproject = Self::read_uproject(path);
        uproject.engine_association = uproject
            .engine_association
            .chars()
            .filter(|c| c != &'{' && c != &'}')
            .collect();
        self_.uproject.replace(Some(uproject));
        if let Some(path) = data.path() {
            let sender = self_.sender.clone();
            thread::spawn(move || {
                Self::load_thumbnail(&path, &sender);
            });
        }
        data
    }

    pub fn guid(&self) -> Option<String> {
        if let Ok(value) = self.property("guid") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn path(&self) -> Option<String> {
        if let Ok(value) = self.property("path") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn name(&self) -> Option<String> {
        if let Ok(value) = self.property("name") {
            if let Ok(id_opt) = value.get::<String>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn image(&self) -> Option<Pixbuf> {
        if let Ok(value) = self.property("thumbnail") {
            if let Ok(id_opt) = value.get::<Pixbuf>() {
                return Some(id_opt);
            }
        };
        None
    }

    pub fn read_uproject(path: &str) -> Uproject {
        let p = std::path::PathBuf::from(path);
        if let Ok(mut file) = std::fs::File::open(p) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return json5::from_str(&contents).unwrap();
            }
        }
        Uproject::default()
    }

    pub fn uproject(&self) -> Option<Uproject> {
        let self_: &imp::ProjectData = imp::ProjectData::from_instance(self);
        self_.uproject.borrow().clone()
    }

    pub fn setup_messaging(&self) {
        let self_: &imp::ProjectData = imp::ProjectData::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as project => @default-panic, move |msg| {
                project.update(msg);
                glib::Continue(true)
            }),
        );
    }

    pub fn update(&self, msg: ProjectMsg) {
        match msg {
            ProjectMsg::Thumbnail(image) => {
                let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&image).unwrap();
                pixbuf_loader.close().ok();

                if let Some(pix) = pixbuf_loader.pixbuf() {
                    self.set_property("thumbnail", &pix).unwrap();
                };
            }
        };
        self.emit_by_name("finished", &[]).unwrap();
    }

    pub fn load_thumbnail(path: &str, sender: &gtk4::glib::Sender<ProjectMsg>) {
        let mut pathbuf = match PathBuf::from(&path).parent() {
            None => return,
            Some(p) => p.to_path_buf(),
        };
        pathbuf.push("Saved");
        pathbuf.push("AutoScreenshot.png");
        match File::open(pathbuf.as_path()) {
            Ok(mut f) => {
                let metadata =
                    std::fs::metadata(&pathbuf.as_path()).expect("unable to read metadata");
                let mut buffer = vec![0; metadata.len() as usize];
                f.read_exact(&mut buffer).expect("buffer overflow");
                let pixbuf_loader = gtk4::gdk_pixbuf::PixbufLoader::new();
                pixbuf_loader.write(&buffer).unwrap();
                pixbuf_loader.close().ok();
                match pixbuf_loader.pixbuf() {
                    None => {}
                    Some(pb) => {
                        let width = pb.width();
                        let height = pb.height();

                        let width_percent = 128.0 / width as f64;
                        let height_percent = 128.0 / height as f64;
                        let percent = if height_percent < width_percent {
                            height_percent
                        } else {
                            width_percent
                        };
                        let desired = (width as f64 * percent, height as f64 * percent);
                        sender
                            .send(ProjectMsg::Thumbnail(
                                pb.scale_simple(
                                    desired.0.round() as i32,
                                    desired.1.round() as i32,
                                    gtk4::gdk_pixbuf::InterpType::Bilinear,
                                )
                                .unwrap()
                                .save_to_bufferv("png", &[])
                                .unwrap(),
                            ))
                            .unwrap();
                    }
                };
            }
            Err(_) => {
                info!("No project file exists for {}", path);
            }
        }
    }
}
