use {
    super::InputFieldProxy,
    eframe::egui::Id,
};

pub trait InputField<'a> {
    type Input;

    fn display(&self, value: Self::Input) -> String;

    fn get_value(&mut self) -> Self::Input {
        (self.proxy())(None)
    }

    fn normalize(&self, value: Self::Input) -> Self::Input;

    fn parse(&self, value_text: &String) -> Option<Self::Input>;

    fn proxy(&mut self) -> &mut InputFieldProxy<'a, Self::Input>;

    fn set_value(&mut self, value_text: &String) -> bool {
        if let value_opt @ Some(_) = self.parse(value_text) {
            (self.proxy())(value_opt);

            true
        } else {
            false
        }
    }
}
